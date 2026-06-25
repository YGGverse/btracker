mod config;
mod full_scrape;
mod opt;
mod tracker;

use anyhow::Result;
use btpeer::http::query::Scrape;
use btracker_fs::crawler::Storage;
use chrono::Local;
use clap::Parser;
use config::Config;
use full_scrape::FullScrape;
use librqbit::{
    AddTorrent, AddTorrentOptions, AddTorrentResponse, ConnectionOptions, Session, SessionOptions,
    limits::LimitsConfig,
};
use log::*;
use opt::Opt;
use regex::Regex;
use std::{collections::HashSet, num::NonZero, time::Duration};
use tokio::time;
use tracker::Tracker;

#[tokio::main]
async fn main() -> Result<()> {
    // debug
    if std::env::var("RUST_LOG").is_ok() {
        use tracing_subscriber::{EnvFilter, fmt::*};
        struct T;
        impl time::FormatTime for T {
            fn format_time(&self, w: &mut format::Writer<'_>) -> std::fmt::Result {
                write!(w, "{}", Local::now())
            }
        }
        fmt()
            .with_env_filter(EnvFilter::from_default_env())
            .with_timer(T)
            .init()
    }
    // init components
    let time_init = Local::now();

    let opt = Opt::parse();
    let config: Config = toml::from_str(&std::fs::read_to_string(&opt.config).unwrap()).unwrap();

    let preload = Storage::init(
        config.preload.path,
        config.preload.regex.map(|r| Regex::new(&r).unwrap()),
        config.preload.max_filecount,
        config.preload.max_filesize,
    )
    .unwrap();

    // init info-hash sources
    let mut scrape = Vec::with_capacity(config.tracker.scrape.len());

    for i in config.tracker.scrape {
        if !i.url.scheme().starts_with("http") {
            todo!("HTTP trackers only!")
        }
        info!("init full scrape source `{}`", i.url);
        scrape.push(FullScrape {
            proxy: i.proxy_url,
            query: Scrape::new(i.url.as_str(), None)?,
            timeout: Duration::from_secs(i.timeout),
        })
    }

    let full_scrape = full_scrape::Buffer(scrape);

    // init trackers (for DHT data preload)
    let mut trackers = Vec::with_capacity(
        config.tracker.announce.len()
            + config
                .tracker
                .announce_i2p
                .as_ref()
                .map(|a| a.len())
                .unwrap_or_default(),
    );

    for i in config.tracker.announce {
        if !i.url.scheme().starts_with("http") {
            todo!("HTTP trackers only!")
        }
        info!("init tracker `{}`", i.url);
        trackers.push(Tracker::Default {
            proxy: i.proxy_url,
            timeout: Duration::from_secs(i.timeout),
            url: i.url,
            port: i.port,
            peers_limit: i.peers_limit,
        })
    }

    if let Some(a) = config.tracker.announce_i2p {
        for i in a {
            if !i.url.scheme().starts_with("http") {
                todo!("HTTP trackers only!")
            }
            info!("init I2P tracker `{}`", i.url);
            trackers.push(Tracker::I2p {
                loopback: i.loopback_host,
                proxy: i.proxy_url,
                timeout: Duration::from_secs(i.timeout),
                inbound_len: i.inbound_len,
                outbound_len: i.outbound_len,
                url: i.url,
                port: i.port,
                peers_limit: i.peers_limit,
            })
        }
    }

    let tracker = tracker::Buffer(trackers);

    // init ban list to skip unresolvable info-hashes between the queue iterations
    let mut ban = HashSet::with_capacity(config.info_hash_capacity);

    // start the crawler
    info!("crawler started at {time_init}");
    loop {
        let time_queue = Local::now();
        debug!("queue crawl begin at {time_queue}...");

        // Please, note:
        // * it's important to start new `Session` inside the crawler loop:
        //   https://github.com/ikatson/rqbit/issues/481
        // * when fix and after starting it once (outside the loop),
        //   remove also each torrent after resolve it with `session.delete`, to prevent impl panic (see `single-session` branch)
        let session = Session::new_with_opts(
            preload.root().clone(),
            SessionOptions {
                bind_device_name: config.bind_device_name.clone(),
                blocklist_url: config.blocklist_url.as_ref().map(|b| b.to_string()),
                listen: None,
                connect: Some(ConnectionOptions {
                    proxy_url: config.proxy_url.as_ref().map(|u| u.to_string()),
                    ..ConnectionOptions::default()
                }),
                dht: None,
                disable_local_service_discovery: true,
                disable_upload: true,
                fastresume: false,
                persistence: None,
                ratelimits: LimitsConfig {
                    download_bps: config.download_limit.and_then(NonZero::new),
                    ..LimitsConfig::default()
                },
                trackers: HashSet::new(), // we're resolving peers manually
                ..SessionOptions::default()
            },
        )
        .await?;
        // build unique ID index from the multiple info-hash sources
        let queue = full_scrape.get(config.info_hash_capacity).await?;
        // clean up nonexistent ban entries from the memory pool
        ban.retain(|i| {
            let is_retain = queue.contains(i);
            if !is_retain {
                debug!(
                    "remove `{}` from the ban list, as it is no longer available in the source.",
                    i.as_string()
                )
            }
            is_retain
        });
        // handle
        debug!(
            "fetched {} unique hashes, banned: {}.",
            queue.len(),
            ban.len()
        );
        for i in queue {
            // convert to string once
            let h = i.as_string();
            if preload.contains_torrent(&h)? {
                debug!("torrent `{h}` exists, skip.");
                continue;
            }

            // skip banned entry, remove it from the ban list to retry on the next iteration
            if ban.remove(&i) {
                debug!("torrent `{h}` is banned, skip.");
                continue;
            }

            debug!("resolve `{h}`...");

            // discover unique peers first
            let initial_peers = match tracker.peers(&i).await {
                Ok(mut peers) => {
                    if let Some(ref p) = config.initial_peers {
                        debug!("forcefully extend with {} peers ({p:?})", p.len());
                        peers.extend(p);
                    }
                    if peers.is_empty() {
                        debug!("could not find peers for torrent `{h}`, skip.");
                        continue;
                    } else {
                        let l = peers.len();
                        debug!("collected {l} peers for torrent `{h}`.");
                        peers
                    }
                }
                Err(e) => {
                    warn!("could not get peers for torrent `{h}`: {e}, skip.");
                    continue;
                }
            };

            // make sure the list is not empty as unexpected here
            assert!(!initial_peers.is_empty());

            // run the crawler in single thread for performance reasons,
            // use `timeout` argument option to skip the dead connections.
            match time::timeout(
                Duration::from_secs(config.timeout.add_torrent_seconds),
                session.add_torrent(
                    AddTorrent::from_url(tracker.magnet(&h)),
                    Some(AddTorrentOptions {
                        paused: true, // continue after `only_files` update
                        overwrite: true,
                        disable_trackers: true, // we're resolving peers manually
                        initial_peers: Some(initial_peers.into_iter().collect()),
                        list_only: preload.regex.is_none(),
                        // the destination folder to preload files match `preload_regex`
                        // * e.g. images for audio albums
                        output_folder: preload.tmp_dir(&h, true)?.to_str().map(|s| s.to_string()),
                        ..Default::default()
                    }),
                ),
            )
            .await
            {
                Ok(r) => match r {
                    Ok(AddTorrentResponse::ListOnly(l)) => {
                        assert!(preload.regex.is_none());
                        debug!("persist bytes for torrent file `{h}`...");
                        preload.commit(&h, l.torrent_bytes.to_vec(), None)?;
                        info!("torrent `{h}` resolved.")
                    }
                    Ok(AddTorrentResponse::Added(_, mt)) => {
                        assert!(preload.regex.is_some());
                        assert!(mt.is_paused());
                        let mut keep_files = HashSet::with_capacity(
                            config.preload.max_filecount.unwrap_or_default(),
                        );
                        let mut only_files = HashSet::with_capacity(
                            config.preload.max_filecount.unwrap_or_default(),
                        );
                        mt.wait_until_initialized().await?;
                        let bytes = mt.with_metadata(|m| {
                                for (id, info) in m.file_infos.iter().enumerate() {
                                    if preload
                                        .max_filecount
                                        .is_some_and(|limit| only_files.len() + 1 > limit)
                                    {
                                        debug!(
                                            "file count limit ({}) reached, skip file `{id}` for `{h}` at `{}` (and other files after it)",
                                            only_files.len(),
                                            info.relative_filename.to_string_lossy()
                                        );
                                        break;
                                    }
                                    if preload.max_filesize.is_some_and(|limit| info.len > limit) {
                                        debug!(
                                            "file size ({}) limit reached, skip file `{id}` for `{h}` at `{}`",
                                            info.len,
                                            info.relative_filename.to_string_lossy()
                                        );
                                        continue;
                                    }
                                    if preload.regex.as_ref().is_some_and(|r| {
                                        !r.is_match(&info.relative_filename.to_string_lossy())
                                    }) {
                                        debug!("regex filter match: skip `{id}` for `{h}` at `{}`",
                                        info.relative_filename.to_string_lossy());
                                        continue;
                                    }
                                    debug!(
                                        "keep file `{id}` for `{h}` as `{}`",
                                        info.relative_filename.to_string_lossy()
                                    );
                                    assert!(keep_files.insert(info.relative_filename.clone()));
                                    assert!(only_files.insert(id))
                                }
                                m.torrent_bytes.to_vec()
                            })?;
                        session.update_only_files(&mt, &only_files).await?;
                        session.unpause(&mt).await?;
                        debug!("begin torrent `{h}` preload...");
                        if let Err(e) = time::timeout(
                            Duration::from_secs(config.timeout.torrent_preload_seconds),
                            mt.wait_until_completed(),
                        )
                        .await
                        {
                            info!(
                                "preload torrent data for `{h}` failed (`{e}`), ban temporarily.",
                            );
                            assert!(ban.insert(i));
                            continue;
                        }
                        debug!("torrent `{h}` preload completed.");
                        // persist torrent bytes and preloaded content,
                        // cleanup tmp (see rqbit#408)
                        debug!("persist torrent `{h}` with `{}` files...", keep_files.len());
                        preload.commit(&h, bytes, Some(keep_files))?;
                        info!("torrent `{h}` resolved.")
                    }
                    Ok(_) => unreachable!(),
                    Err(e) => {
                        debug!("failed to resolve torrent `{h}`: `{e}`, ban temporarily.");
                        assert!(ban.insert(i))
                    }
                },
                Err(e) => {
                    info!(
                        "skip awaiting the completion of adding torrent `{h}` (`{e}`), ban temporarily."
                    );
                    assert!(ban.insert(i))
                }
            }
        }
        session.stop().await;
        info!(
            "queue completed at {time_queue} (time: {} / uptime: {} / banned: {}) await {} seconds to continue...",
            Local::now()
                .signed_duration_since(time_queue)
                .as_seconds_f32(),
            Local::now()
                .signed_duration_since(time_init)
                .as_seconds_f32(),
            ban.len(),
            config.sleep_seconds
        );
        std::thread::sleep(Duration::from_secs(config.sleep_seconds))
    }
}
