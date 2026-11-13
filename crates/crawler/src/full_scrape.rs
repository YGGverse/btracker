use anyhow::Result;
use btpeer::{InfoHash, http::query::Scrape, udp::Server};
use librqbit::dht::Id20;
use log::*;
use std::{
    collections::HashSet,
    net::{SocketAddr, UdpSocket},
    time::Duration,
};
use url::Url;

pub enum FullScrape {
    Http {
        headers: Vec<String>,
        proxy: Option<Url>,
        query: Scrape,
        timeout: Duration,
    },
    Udp {
        server: UdpSocket,
        socket: SocketAddr,
    },
}

pub struct Buffer(pub Vec<FullScrape>);

impl Buffer {
    pub async fn get(&self, expected_capacity: usize) -> Result<HashSet<Id20>> {
        let mut s = HashSet::with_capacity(expected_capacity);

        for this in self.0.iter() {
            match this {
                FullScrape::Http {
                    headers,
                    proxy,
                    query,
                    timeout,
                } => {
                    let scrape = match btpeer::http::scrape(
                        query,
                        *timeout,
                        headers,
                        proxy.as_ref().map(|u| u.as_str()),
                    )
                    .await
                    {
                        Ok(result) => result,
                        Err(e) => {
                            warn!("[full-scrape] full-scrape {query} update failed: {e}; skip",);
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
                        query.0
                    )
                }
                FullScrape::Udp { server, socket } => {
                    //let s =  server.full_scrape(socket)?;
                    todo!("UDP trackers yet not supported")
                }
            }
        }

        debug!(
            "[full-scrape] collected {} unique hashes to crawl...",
            s.len()
        );

        Ok(s)
    }
}
