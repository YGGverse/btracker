mod tracker;

use serde::Deserialize;
use serde_inline_default::serde_inline_default;
use std::{net::SocketAddr, path::PathBuf};
use tracker::Tracker;
use url::Url;

#[derive(Deserialize)]
pub struct Preload {
    /// Directory path to store preloaded data (e.g. `.torrent` files)
    ///
    /// * it's probably the same location as `public` dir for the `btracker-http` frontend
    pub path: PathBuf,

    /// Preload content file (names) match regex pattern (.torrent file only if `None`)
    /// * see also `preload_max_filesize`, `preload_max_filecount` options
    ///
    /// ## Example:
    ///
    /// ```
    /// \.(png|gif|jpeg|jpg|webp|svg|log|nfo|txt)$
    /// ```
    pub regex: Option<String>,

    /// Max size sum of preloaded files per torrent (match `preload_regex`)
    pub max_filesize: Option<u64>,

    /// Max count of preloaded files per torrent (match `preload_regex`)
    pub max_filecount: Option<usize>,
}

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

    /// Disable TCP connection
    #[serde_inline_default(true)]
    pub disable_tcp: bool,

    /// Limit download speed (b/s)
    pub download_limit: Option<u32>,

    /// Define initial peer(s) to preload the `.torrent` files info
    pub initial_peers: Option<Vec<SocketAddr>>,

    /// Use `socks5://[username:password@]host:port` for librqbit connections
    pub proxy_url: Option<Url>,

    /// The P2P Blocklist file URL (to filter outgoing connections)
    ///
    /// * e.g. `file:///path/to/blocklist.txt` for local file
    pub blocklist: Option<Url>,

    /// Skip and ban slow or unresolvable hashes
    /// when the specified value in seconds is reached
    ///
    /// * the ban time is dynamically calculated based on the current ban list collected
    /// * tip: increase this value when using I2P features
    #[serde_inline_default(60)]
    pub timeout_seconds: u64,
}
