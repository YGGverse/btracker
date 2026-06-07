mod config;
mod full_scrape;

#[cfg(feature = "i2p")]
mod i2p;

use anyhow::Result;
use librqbit::{
    AddTorrent, AddTorrentOptions, AddTorrentResponse, ConnectionOptions, DhtSessionConfig,
    Session, SessionOptions,
};
use std::{collections::HashSet, num::NonZero, time::Duration};
use url::Url;

#[tokio::main]
async fn main() -> Result<()> {
    use btracker_fs::crawler::Storage;
    use chrono::Local;
    use clap::Parser;
    use config::Config;
    use log::*;
    use tokio::time;
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
    let config = Config::parse();
    let preload = Storage::init(
        config.preload,
        config.preload_regex,
        config.preload_max_filecount,
        config.preload_max_filesize,
    )
    .unwrap();

    let mut ban = HashSet::with_capacity(config.index_capacity);
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
                bind_device_name: config.bind.clone(),
                blocklist_url: config.blocklist.as_ref().map(|b| b.to_string()),
                listen: None,
                connect: Some(ConnectionOptions {
                    enable_tcp: !config.disable_tcp,
                    proxy_url: config.proxy_url.as_ref().map(|u| u.to_string()),
                    ..ConnectionOptions::default()
                }),
                dht: if config.enable_dht {
                    Some(DhtSessionConfig {
                        persistence: None,
                        ..DhtSessionConfig::default()
                    })
                } else {
                    None
                },
                disable_local_service_discovery: !config.enable_lsd,
                disable_upload: true,
                fastresume: false,
                persistence: None,
                ratelimits: librqbit::limits::LimitsConfig {
                    download_bps: config.download_limit.and_then(NonZero::new),
                    ..librqbit::limits::LimitsConfig::default()
                },
                trackers: config.tracker.iter().cloned().collect(),
                ..SessionOptions::default()
            },
        )
        .await?;

        // build unique ID index from the multiple info-hash sources
        let mut queue = HashSet::with_capacity(config.index_capacity);
        for source in &config.full_scrape {
            debug!("index source `{source}`...");
            for i in match full_scrape::get(
                source,
                config.index_capacity,
                Duration::from_secs(config.full_scrape_timeout),
                &config.full_scrape_compression,
                None,
            )
            .await
            {
                Ok(i) => {
                    debug!("fetch `{}` hashes from `{source}`...", i.len());
                    i
                }
                Err(e) => {
                    warn!("the full scrape `{source}` update failed: `{e}`; skip.");
                    continue; // skip without panic
                }
            } {
                queue.insert(i);
            }
        }
        #[cfg(feature = "i2p")]
        {
            for source in &config.i2p_full_scrape {
                debug!("index I2P source `{source}`...");
                for i in match full_scrape::get(
                    source,
                    config.index_capacity,
                    Duration::from_secs(config.i2p_full_scrape_timeout),
                    &config.i2p_full_scrape_compression,
                    config.i2p_proxy.as_ref().map(|p| p.as_str()),
                )
                .await
                {
                    Ok(i) => {
                        debug!("fetch `{}` hashes from I2P `{source}`...", i.len());
                        i
                    }
                    Err(e) => {
                        warn!("I2P full scrape `{source}` update failed: `{e}`; skip.");
                        continue; // skip without panic
                    }
                } {
                    queue.insert(i);
                }
            }
        }

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
            "fetched {} unique hashes from {} source, banned: {}.",
            queue.len(),
            config.full_scrape.len(),
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
                debug!("torrent `{h}` is banned, skip for this queue.");
                continue;
            }
            // init unique peers hash table

            info!("resolve `{h}`...");
            // run the crawler in single thread for performance reasons,
            // use `timeout` argument option to skip the dead connections.
            match time::timeout(
                Duration::from_secs(config.timeout),
                session.add_torrent(
                    AddTorrent::from_url(magnet(
                        &h,
                        if config.tracker.is_empty() {
                            None
                        } else {
                            Some(config.tracker.as_ref())
                        },
                    )),
                    Some(AddTorrentOptions {
                        paused: true, // continue after `only_files` update
                        overwrite: true,
                        disable_trackers: config.tracker.is_empty(),
                        initial_peers: {
                            #[cfg(feature = "i2p")]
                            {
                                let mut peers: HashSet<std::net::SocketAddr> = config
                                    .initial_peer
                                    .as_ref()
                                    .map(|p| p.iter().cloned().collect())
                                    .unwrap_or_default();
                                match i2p::get_peers(
                                    &i.0,
                                    &config.i2p_tracker,
                                    config.i2p_tracker_announce_timeout,
                                    config.i2p_proxy.as_ref(),
                                )
                                .await
                                {
                                    Ok(p) => peers.extend(p),
                                    Err(e) => warn!("{e}"),
                                }
                                if peers.is_empty() {
                                    None
                                } else {
                                    trace!("Collected {} unique peers to handle", peers.len());
                                    Some(peers.into_iter().collect())
                                }
                            }
                            #[cfg(not(feature = "i2p"))]
                            {
                                config.initial_peer.clone()
                            }
                        },
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
                            config.preload_max_filecount.unwrap_or_default(),
                        );
                        let mut only_files = HashSet::with_capacity(
                            config.preload_max_filecount.unwrap_or_default(),
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
                            Duration::from_secs(config.timeout),
                            mt.wait_until_completed(),
                        )
                        .await
                        {
                            info!(
                                "skip awaiting the completion of preload torrent data for `{h}` (`{e}`), ban for the next queue.",
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
                        warn!("failed to resolve torrent `{h}`: `{e}`, ban for the next queue.");
                        assert!(ban.insert(i))
                    }
                },
                Err(e) => {
                    info!(
                        "skip awaiting the completion of adding torrent `{h}` (`{e}`), ban for the next queue."
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
            config.sleep
        );
        std::thread::sleep(Duration::from_secs(config.sleep))
    }
}

/// Build magnet URI (`librqbit` impl dependency)
fn magnet(info_hash: &str, trackers: Option<&Vec<Url>>) -> String {
    let mut m = format!("magnet:?xt=urn:btih:{info_hash}");
    if let Some(t) = trackers {
        for tracker in t {
            m.push_str("&tr=");
            m.push_str(&urlencoding::encode(tracker.as_str()))
        }
    }
    m
}
