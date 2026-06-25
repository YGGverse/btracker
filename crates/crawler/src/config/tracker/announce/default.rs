use serde::Deserialize;
use serde_inline_default::serde_inline_default;
use url::Url;

/// Peers source
#[serde_inline_default]
#[derive(Deserialize)]
pub struct Default {
    /// URL to announce
    ///
    /// * supports HTTP trackers only
    pub url: Url,

    /// How long to wait for tracker full scrape response
    #[serde_inline_default(5)]
    pub timeout: u64,

    /// Static port for outgoing announce connections
    #[serde_inline_default(6699)]
    pub port: u16,

    /// Use HTTP(s) proxy, e.g. `http://127.0.0.1:9050` or `http://127.0.0.1:4444` for I2P
    pub proxy_url: Option<Url>,

    /// Max peers per tracker
    pub peers_limit: Option<usize>,
}
