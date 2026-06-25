mod preload;
mod tracker;

use preload::Preload;
use serde::Deserialize;
use serde_inline_default::serde_inline_default;
use std::net::SocketAddr;
use tracker::Tracker;
use url::Url;

#[serde_inline_default]
#[derive(Deserialize)]
pub struct Config {
    /// Preload data config
    pub preload: Preload,

    /// Estimated info-hash index capacity
    ///
    /// * use for memory optimization, depending on tracker volumes
    #[serde_inline_default(1000)]
    pub info_hash_capacity: usize,

    /// Crawl loop delay in seconds
    #[serde_inline_default(60)]
    pub sleep_seconds: u64,

    /// Tracker settings
    pub tracker: Tracker,

    /// Bind librqbit session on specified device name (`tun0`, `mycelium`, etc.)
    pub bind_device_name: Option<String>,

    /// Limit download speed (b/s)
    pub download_limit: Option<u32>,

    /// Define initial peer(s) to preload the `.torrent` files info
    pub initial_peers: Option<Vec<SocketAddr>>,

    /// Use `socks5://[username:password@]host:port` for librqbit connections
    pub proxy_url: Option<Url>,

    /// The P2P Blocklist file URL (to filter outgoing connections)
    ///
    /// * e.g. `file:///path/to/blocklist.txt` for local file
    pub blocklist_url: Option<Url>,

    #[serde_inline_default(60)]
    pub timeout_add_torrent_seconds: u64,

    #[serde_inline_default(60)]
    pub timeout_torrent_preload_seconds: u64,
}
