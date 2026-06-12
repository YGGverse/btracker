use anyhow::{Result as R, bail};
use btpeer::http::response::scrape::Total;
use librqbit_core::Id20;
use std::time::Duration;
use url::Url;

pub type Result = Total;

struct Scrape {
    proxy: Option<String>,
    timeout: Duration,
    tracker: Url,
}

impl Scrape {
    pub fn new(
        tracker: Url,
        timeout: u64,
        proxy: Option<String>,
        proxy_i2p: Option<String>,
    ) -> R<Self> {
        if !tracker.scheme().starts_with("http") {
            bail!("HTTP trackers only!")
        }
        Ok(Self {
            proxy: if tracker
                .host_str()
                .expect("Host is required")
                .ends_with(".i2p")
            {
                if proxy_i2p.is_none() {
                    bail!("I2P proxy is required for tracker `{tracker}`")
                }
                proxy_i2p
            } else {
                proxy
            },
            timeout: Duration::from_secs(timeout),
            tracker,
        })
    }

    pub async fn get(&self, id20: Id20) -> R<Total> {
        Ok(btpeer::http::scrape(
            &btpeer::http::query::Scrape::new(self.tracker.as_str(), Some(&[id20.0]))?,
            self.timeout,
            self.proxy.as_deref(),
        )
        .await?
        .total)
    }
}

pub struct Buffer(Vec<Scrape>);

impl Buffer {
    pub fn new(
        trackers: Vec<Url>,
        timeout: u64,
        proxy: Option<&Url>,
        proxy_i2p: Option<&Url>,
    ) -> R<Self> {
        let mut this = Vec::with_capacity(trackers.len());

        for url in trackers {
            this.push(Scrape::new(
                url,
                timeout,
                proxy.as_ref().map(|p| p.to_string()),
                proxy_i2p.as_ref().map(|p| p.to_string()),
            )?)
        }

        Ok(Self(this))
    }

    pub async fn get(&self, id20: Id20) -> R<Total> {
        let mut total = Total::default();

        for this in self.0.iter() {
            let result = this.get(id20).await?;
            total.complete += result.complete;
            total.downloaded += result.downloaded;
            total.incomplete += result.incomplete;
        }

        Ok(total)
    }
}
