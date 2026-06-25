use serde::Deserialize;
use serde_inline_default::serde_inline_default;
use std::net::{IpAddr, Ipv4Addr};
use url::Url;

/// Peers source I2P
#[serde_inline_default]
#[derive(Deserialize)]
pub struct I2p {
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

    /// Bind I2P / SAM bridge on given host
    ///
    /// * only if the I2P `full_scrape` trackers in use
    #[serde_inline_default(IpAddr::V4(Ipv4Addr::LOCALHOST))]
    pub loopback: IpAddr,

    /// How many hops do the inbound tunnels of the session have
    #[serde_inline_default(3)]
    pub inbound_len: usize,

    /// How many hops do the outbound tunnels of the session have
    #[serde_inline_default(3)]
    pub outbound_len: usize,

    /// Max peers per tracker
    pub peers_limit: Option<usize>,
}
