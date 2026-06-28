use anyhow::Result;
use btpeer::{InfoHash, http::query::Scrape};
use librqbit::dht::Id20;
use log::*;
use std::{collections::HashSet, time::Duration};
use url::Url;

pub struct FullScrape {
    pub proxy: Option<Url>,
    pub query: Scrape,
    pub timeout: Duration,
}

pub struct Buffer(pub Vec<FullScrape>);

impl Buffer {
    pub async fn get(&self, expected_capacity: usize) -> Result<HashSet<Id20>> {
        let mut s = HashSet::with_capacity(expected_capacity);

        for this in self.0.iter() {
            let scrape = match btpeer::http::scrape(
                &this.query,
                this.timeout,
                this.proxy.as_ref().map(|u| u.as_str()),
            )
            .await
            {
                Ok(result) => result,
                Err(e) => {
                    warn!(
                        "[full-scrape] full-scrape {} update failed: {e}; skip",
                        &this.query
                    );
                    continue; // skip without panic
                }
            }
            .stats
            .into_keys();

            let total = scrape.len();

            for i in scrape {
                s.insert(match i {
                    InfoHash::V1(ref b) => Id20::from_bytes(b)?,
                });
            }

            debug!(
                "[full-scrape] received {total} unique hashes from {}...",
                this.query.0
            )
        }

        debug!(
            "[full-scrape] collected {} unique hashes to crawl...",
            s.len()
        );

        Ok(s)
    }
}
