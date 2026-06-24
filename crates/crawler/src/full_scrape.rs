use anyhow::{Result, bail};
use btpeer::{InfoHash, http::query::Scrape};
use librqbit::dht::Id20;
use log::*;
use std::{collections::HashSet, time::Duration};
use url::Url;

struct FullScrape {
    proxy: Option<String>,
    query: Scrape,
    timeout: Duration,
}

impl FullScrape {
    pub fn new(
        tracker: Url,
        timeout: u64,
        proxy: Option<String>,
        proxy_i2p: Option<String>,
    ) -> Result<Self> {
        if !tracker.scheme().starts_with("http") {
            bail!("HTTP trackers only!")
        }
        Ok(Self {
            proxy: if tracker.host_str().unwrap().ends_with(".i2p") {
                if proxy_i2p.is_none() {
                    bail!("I2P proxy is required for tracker `{tracker}`")
                }
                info!(
                    "[full-scrape] init full-scrape source `{tracker}` using proxy {}",
                    proxy_i2p.as_ref().unwrap()
                );
                proxy_i2p
            } else {
                info!("[full-scrape] init full-scrape source `{tracker}` using {proxy:?} proxy ");
                proxy
            },
            query: Scrape::new(tracker.as_str(), None)?,
            timeout: Duration::from_secs(timeout),
        })
    }
}

pub struct Buffer(Vec<FullScrape>);

impl Buffer {
    pub fn new(
        trackers: Vec<Url>,
        timeout: u64,
        proxy: Option<&Url>,
        proxy_i2p: Option<&Url>,
    ) -> Result<Self> {
        let mut b = Vec::with_capacity(trackers.len());
        for url in trackers {
            debug!("[full-scrape] index full-scrape source `{url}`...");
            b.push(FullScrape::new(
                url,
                timeout,
                proxy.as_ref().map(|p| p.to_string()),
                proxy_i2p.as_ref().map(|p| p.to_string()),
            )?)
        }
        Ok(Self(b))
    }

    pub async fn get(&self, expected_capacity: usize) -> Result<HashSet<Id20>> {
        let mut b = HashSet::with_capacity(expected_capacity);

        for this in self.0.iter() {
            for i in
                match btpeer::http::scrape(&this.query, this.timeout, this.proxy.as_deref()).await {
                    Ok(result) => result,
                    Err(e) => {
                        warn!(
                            "[full-scrape] the full-scrape `{}` update failed: `{e}`; skip.",
                            &this.query
                        );
                        continue; // skip without panic}
                    }
                }
                .stats
                .into_keys()
            {
                b.insert(match i {
                    InfoHash::V1(ref b) => Id20::from_bytes(b)?,
                });
            }
        }

        debug!(
            "[full-scrape] collected {} unique hashes to crawl...",
            b.len()
        );

        Ok(b)
    }
}
