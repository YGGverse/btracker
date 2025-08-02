mod api;
mod config;
mod peers;
mod preload;
mod trackers;

use anyhow::Result;
use config::Config;
use librqbit::{
    AddTorrent, AddTorrentOptions, AddTorrentResponse, ConnectionOptions, PeerConnectionOptions,
    SessionOptions,
};
use peers::Peers;
use preload::Preload;
use std::{
    collections::HashSet, num::NonZero, os::unix::ffi::OsStrExt, path::PathBuf, time::Duration,
};
use trackers::Trackers;
use url::Url;
use yggtracker_redb::{
    Database,
    torrent::{Torrent, meta::*},
};

#[tokio::main]
async fn main() -> Result<()> {
    use chrono::Local;
    use clap::Parser;
    use tokio::time;

    // init components
    let time_init = Local::now();
    let config = Config::parse();
    if std::env::var("RUST_LOG").is_ok() {
        tracing_subscriber::fmt::init()
    } // librqbit impl dependency
    let database = Database::init(&config.database)?;
    let peers = Peers::init(&config.initial_peer)?;
    let preload = Preload::init(
        config.preload,
        config.preload_max_filecount,
        config.preload_max_filesize,
    )?;
    let trackers = Trackers::init(&config.tracker)?;
    let session = librqbit::Session::new_with_opts(
        preload.root(),
        SessionOptions {
            bind_device_name: config.bind,
            listen: None,
            connect: Some(ConnectionOptions {
                enable_tcp: true,
                proxy_url: config.proxy_url,
                peer_opts: Some(PeerConnectionOptions {
                    connect_timeout: config.peer_connect_timeout.map(Duration::from_secs),
                    read_write_timeout: config.peer_read_write_timeout.map(Duration::from_secs),
                    keep_alive_interval: config.peer_keep_alive_interval.map(Duration::from_secs),
                }),
            }),
            disable_upload: false,
            disable_dht: !config.enable_dht,
            disable_dht_persistence: true,
            persistence: None,
            ratelimits: librqbit::limits::LimitsConfig {
                upload_bps: config.upload_limit.and_then(NonZero::new),
                download_bps: config.download_limit.and_then(NonZero::new),
            },
            trackers: trackers.list().clone(),
            ..SessionOptions::default()
        },
    )
    .await?;

    // begin
    println!("Crawler started on {time_init}");
    loop {
        let time_queue = Local::now();
        if config.debug {
            println!("\tQueue crawl begin on {time_queue}...")
        }
        for source in &config.infohash {
            if config.debug {
                println!("\tIndex source `{source}`...")
            }
            // grab latest info-hashes from this source
            // * aquatic server may update the stats at this moment, handle result manually
            for i in match api::get(source, config.index_capacity) {
                Some(i) => i,
                None => {
                    // skip without panic
                    if config.debug {
                        eprintln!(
                            "The feed `{source}` has an incomplete format (or is still updating); skip."
                        )
                    }
                    continue;
                }
            } {
                // convert to string once
                let i = i.to_string();
                // already indexed?
                if database.has_torrent(&i)? {
                    continue;
                }
                if config.debug {
                    println!("\t\tIndex `{i}`...")
                }
                // run the crawler in single thread for performance reasons,
                // use `timeout` argument option to skip the dead connections.
                match time::timeout(
                    Duration::from_secs(config.add_torrent_timeout),
                    session.add_torrent(
                        AddTorrent::from_url(magnet(
                            &i,
                            if config.export_trackers && !trackers.is_empty() {
                                Some(trackers.list())
                            } else {
                                None
                            },
                        )),
                        Some(AddTorrentOptions {
                            paused: true, // continue after `only_files` init
                            overwrite: true,
                            disable_trackers: trackers.is_empty(),
                            initial_peers: peers.initial_peers(),
                            list_only: false, // we want to grab the images
                            // it is important to blacklist all files preload until initiation
                            only_files: Some(Vec::with_capacity(
                                config.preload_max_filecount.unwrap_or_default(),
                            )),
                            // the folder to preload temporary files (e.g. images for the audio albums)
                            output_folder: Some(preload.output_folder(&i)?),
                            ..Default::default()
                        }),
                    ),
                )
                .await
                {
                    Ok(r) => match r {
                        Ok(AddTorrentResponse::Added(id, mt)) => {
                            let mut images: HashSet<PathBuf> = HashSet::with_capacity(
                                config
                                    .preload_max_filecount
                                    .unwrap_or(config.index_capacity),
                            );
                            let mut only_files: HashSet<usize> = HashSet::with_capacity(
                                config
                                    .preload_max_filecount
                                    .unwrap_or(config.index_capacity),
                            );
                            mt.wait_until_initialized().await?;
                            let (name, files, is_private, length, bytes) = mt.with_metadata(|m| {
                                for info in &m.file_infos {
                                    if preload.max_filecount.is_some_and(|limit| images.len() + 1 > limit) {
                                        if config.debug {
                                            println!(
                                                "\t\t\ttotal files count limit ({}) for `{i}` reached!",
                                                preload.max_filecount.unwrap()
                                            )
                                        }
                                        break;
                                    }
                                    if info.relative_filename.extension().is_none_or(|e|
                                        !matches!(e.as_bytes(), b"png" | b"jpeg" | b"jpg" | b"gif" | b"webp")) {
                                        continue;
                                    }
                                    if preload.max_filesize.is_some_and(|limit| info.len > limit) {
                                        if config.debug {
                                            println!(
                                                "\t\t\ttotal files size limit `{i}` reached!"
                                            )
                                        }
                                        continue;
                                    }
                                    assert!(images.insert(info.relative_filename.clone()));
                                    assert!(only_files.insert(id))
                                }
                                let mi = m.info.info();
                                (
                                    mi.name.as_ref().map(|s| s.to_string()),
                                    mi.files.as_ref().map(|f| {
                                        let mut b = Vec::with_capacity(f.len());
                                        let mut i = f.iter();
                                        for f in i.by_ref() {
                                            b.push(File {
                                                path: String::from_utf8(
                                                    f.path
                                                        .iter()
                                                        .enumerate()
                                                        .flat_map(|(n, b)| {
                                                            if n == 0 {
                                                                b.0.to_vec()
                                                            } else {
                                                                let mut p = vec![b'/'];
                                                                p.extend(b.0.to_vec());
                                                                p
                                                            }
                                                        })
                                                        .collect(),
                                                )
                                                .ok(),
                                                length: f.length,
                                            });
                                        }
                                        b.sort_by(|a, b| a.path.cmp(&b.path)); // @TODO optional
                                        b
                                    }),
                                    mi.private,
                                    mi.length,
                                    m.torrent_bytes.clone().into()
                                )
                            })?;
                            session.update_only_files(&mt, &only_files).await?;
                            session.unpause(&mt).await?;
                            mt.wait_until_completed().await?;
                            assert!(
                                database
                                    .set_torrent(
                                        &i,
                                        Torrent {
                                            bytes,
                                            meta: Meta {
                                                comment: None, // @TODO
                                                files,
                                                images: if images.is_empty() {
                                                    None
                                                } else {
                                                    let mut b = Vec::with_capacity(images.len());
                                                    for p in images {
                                                        b.push(Image {
                                                            bytes: preload.bytes(&p)?,
                                                            path: p.to_string_lossy().to_string(),
                                                        })
                                                    }
                                                    Some(b)
                                                },
                                                is_private,
                                                name,
                                                length,
                                                time: chrono::Utc::now(),
                                            },
                                        },
                                    )?
                                    .is_none()
                            );
                            // remove torrent from session as indexed
                            session
                                .delete(librqbit::api::TorrentIdOrHash::Id(id), false)
                                .await?;

                            // cleanup `output_folder` only if the torrent is resolved
                            // to prevent extra write operations on the next iteration
                            preload.clear_output_folder(&i)?;

                            if config.debug {
                                println!("\t\t\tadd `{i}` to index.")
                            }
                        }
                        Ok(_) => panic!(),
                        Err(e) => eprintln!("Failed to resolve `{i}`: `{e}`."),
                    },
                    Err(e) => {
                        if config.debug {
                            println!("\t\t\tfailed to resolve `{i}`: `{e}`")
                        }
                    }
                }
            }
        }
        if config.debug {
            println!(
                "Queue completed on {time_queue}\n\ttotal: {}\n\ttime: {} s\n\tuptime: {} s\n\tawait {} seconds to continue...",
                database.torrents_total()?,
                Local::now()
                    .signed_duration_since(time_queue)
                    .as_seconds_f32(),
                Local::now()
                    .signed_duration_since(time_init)
                    .as_seconds_f32(),
                config.sleep,
            )
        }
        std::thread::sleep(Duration::from_secs(config.sleep))
    }
}

/// Build magnet URI
fn magnet(infohash: &str, trackers: Option<&HashSet<Url>>) -> String {
    let mut m = if infohash.len() == 40 {
        format!("magnet:?xt=urn:btih:{infohash}")
    } else {
        todo!("infohash v2 is not supported by librqbit")
    };
    if let Some(t) = trackers {
        for tracker in t {
            m.push_str("&tr=");
            m.push_str(&urlencoding::encode(tracker.as_str()))
        }
    }
    m
}
