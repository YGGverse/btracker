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
use libyggtracker_redb::{
    Database,
    torrent::{Image, Torrent, image},
};
use peers::Peers;
use preload::Preload;
use std::{collections::HashSet, num::NonZero, os::unix::ffi::OsStrExt, time::Duration};
use trackers::Trackers;
use url::Url;

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
        preload.root().clone(),
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
                            output_folder: Some(
                                preload.output_folder(&i)?.to_string_lossy().to_string(),
                            ),
                            ..Default::default()
                        }),
                    ),
                )
                .await
                {
                    Ok(r) => match r {
                        Ok(AddTorrentResponse::Added(id, mt)) => {
                            let mut only_files = HashSet::with_capacity(
                                config
                                    .preload_max_filecount
                                    .unwrap_or(config.index_capacity),
                            );
                            let mut images = Vec::with_capacity(
                                config
                                    .preload_max_filecount
                                    .unwrap_or(config.index_capacity),
                            );
                            mt.wait_until_initialized().await?;
                            let bytes = mt.with_metadata(|m| {
                                for info in &m.file_infos {
                                    if preload.max_filecount.is_some_and(|limit| only_files.len() + 1 > limit) {
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
                                    assert!(only_files.insert(id));
                                    images.push(info.relative_filename.clone());
                                }
                                m.info_bytes.to_vec()
                            })?;
                            session.update_only_files(&mt, &only_files).await?;
                            session.unpause(&mt).await?;
                            mt.wait_until_completed().await?;

                            // persist torrent data resolved
                            database.set_torrent(
                                &i,
                                Torrent {
                                    bytes,
                                    images: if images.is_empty() {
                                        None
                                    } else {
                                        Some(
                                            images
                                                .into_iter()
                                                .filter_map(|p| {
                                                    extension(&p).map(|extension| Image {
                                                        alt: p.to_str().map(|s| s.to_string()),
                                                        bytes: preload.bytes(&p).unwrap(),
                                                        extension,
                                                    })
                                                })
                                                .collect(),
                                        )
                                    },
                                    time: chrono::Utc::now(),
                                },
                            )?;

                            // remove torrent from session as indexed
                            session
                                .delete(librqbit::api::TorrentIdOrHash::Id(id), false)
                                .await?;

                            // cleanup `output_folder` only if the torrent is resolved
                            // to prevent extra write operations on the next iteration
                            preload.clear_output_folder(&i)?;

                            if config.debug {
                                println!("\t\t\ttorrent data successfully resolved.")
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
                "Queue completed on {time_queue}\n\ttime: {} s\n\tuptime: {} s\n\tawait {} seconds to continue...",
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

use image::Extension;
fn extension(path: &std::path::Path) -> Option<Extension> {
    match path.extension() {
        Some(p) => {
            let e = p.to_string_lossy().to_lowercase();
            if e == "png" {
                Some(Extension::Png)
            } else if e == "jpeg" || e == "jpg" {
                Some(Extension::Jpeg)
            } else if e == "webp" {
                Some(Extension::Webp)
            } else if e == "gif" {
                Some(Extension::Gif)
            } else {
                return None;
            }
        }
        None => None,
    }
}
