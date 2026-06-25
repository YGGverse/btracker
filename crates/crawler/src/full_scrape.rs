use anyhow::Result;
use btpeer::{InfoHash, http::query::Scrape};
use librqbit::dht::Id20;
use log::*;
use std::{collections::HashSet, time::Duration};

pub struct FullScrape {
    pub proxy: Option<String>,
    pub query: Scrape,
    pub timeout: Duration,
}

pub struct Buffer(pub Vec<FullScrape>);

impl Buffer {
    pub async fn get(&self, expected_capacity: usize) -> Result<HashSet<Id20>> {
        let mut b = HashSet::with_capacity(expected_capacity);

        for this in self.0.iter() {
            for i in
                match btpeer::http::scrape(&this.query, this.timeout, this.proxy.as_deref()).await {
                    Ok(result) => result,
                    Err(e) => {
                        warn!(
                            "[full-scrape] full-scrape `{}` update failed: `{e}`; skip",
                            &this.query
                        );
                        continue; // skip without panic
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
