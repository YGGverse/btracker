use clap::Parser;
use regex::Regex;
use std::{net::SocketAddr, path::PathBuf};
use url::Url;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Config {
    /// Directory path to store preloaded data (e.g. `.torrent` files)
    ///
    /// * it's probably the same location as `public` dir for the `btracker-http` frontend
    #[arg(long, short)]
    pub preload: PathBuf,

    /// Absolute path(s) or URL(s) to the BEP 48 / Full Scrape
    #[arg(long, short)]
    pub full_scrape: Vec<Url>,

    /// How long to wait for tracker full scrape response
    /// * tip: by using OpenTracker,
    ///   make sure `FEATURES+=-DWANT_FULLSCRAPE` is enabled!
    #[arg(long, default_value_t = 15)]
    pub full_scrape_timeout: u64,

    /// Use HTTP(s) proxy to resolve `full_scrape` trackers, would be `http://127.0.0.1:9050`
    #[arg(long)]
    pub full_scrape_proxy: Option<Url>,

    /// Use HTTP(s) proxy to resolve `full_scrape` I2P trackers, would be `http://127.0.0.1:4444`
    #[arg(long)]
    pub full_scrape_proxy_i2p: Option<Url>,

    /// The P2P Blocklist file URL (to filter outgoing connections)
    ///
    /// * use `--blocklist=file:///path/to/blocklist.txt` format for the local path
    #[arg(long)]
    pub blocklist: Option<Url>,

    /// Define HTTP tracker(s) to preload the `.torrent` files info
    #[arg(long, short)]
    pub tracker_announce: Vec<Url>,

    /// Announce timeout for every info-hash handle
    /// * increase by using I2P trackers, but keep in mind about global `timeout`
    #[arg(long, default_value_t = 15)]
    pub tracker_announce_timeout: u64,

    /// Static port for outgoing announce connections
    #[arg(long, default_value_t = 6699)]
    pub tracker_announce_port: u16,

    /// Use HTTP(s) proxy to resolve `full_scrape` trackers, would be `http://127.0.0.1:9050`
    #[arg(long)]
    pub tracker_announce_proxy: Option<Url>,

    /// Use HTTP(s) proxy to resolve `full_scrape` I2P trackers, would be `http://127.0.0.1:4444`
    #[arg(long)]
    pub tracker_announce_proxy_i2p: Option<Url>,

    /// Define initial peer(s) to preload the `.torrent` files info
    #[arg(long)]
    pub initial_peer: Option<Vec<SocketAddr>>,

    /// Appends `--tracker` value to magnets and torrents
    #[arg(long, default_value_t = false)]
    pub export_trackers: bool,

    /// Enable DHT resolver
    #[arg(long, default_value_t = false)]
    pub enable_dht: bool,

    /// Enable LSD multicast
    #[arg(long, default_value_t = false)]
    pub enable_lsd: bool,

    /// Disable TCP connection
    #[arg(long, default_value_t = false)]
    pub disable_tcp: bool,

    /// Bind resolver session on specified device name (`tun0`, `mycelium`, etc.)
    #[arg(long)]
    pub bind: Option<String>,

    /// Preload content file (names) match regex pattern (.torrent file only if `None`)
    /// * see also `preload_max_filesize`, `preload_max_filecount` options
    ///
    /// ## Example:
    ///
    /// Filter by image ext
    /// ```
    /// --preload-regex '\.(png|gif|jpeg|jpg|webp|svg|log|nfo|txt)$'
    /// ```
    #[arg(long)]
    pub preload_regex: Option<Regex>,

    /// Max size sum of preloaded files per torrent (match `preload_regex`)
    #[arg(long)]
    pub preload_max_filesize: Option<u64>,

    /// Max count of preloaded files per torrent (match `preload_regex`)
    #[arg(long)]
    pub preload_max_filecount: Option<usize>,

    /// Limit download speed (b/s)
    #[arg(long)]
    pub download_limit: Option<u32>, // * reminder: upload feature is not planed by the crawler impl

    /// Use `socks5://[username:password@]host:port` for standard torrent connections
    #[arg(long)]
    pub proxy: Option<Url>,

    /// Estimated info-hash index capacity
    ///
    /// * use for memory optimization, depending on tracker volumes
    #[arg(long, default_value_t = 1000)]
    pub index_capacity: usize,

    /// Crawl loop delay in seconds
    #[arg(long, default_value_t = 60)]
    pub sleep: u64,

    /// Skip and ban slow or unresolvable hashes
    /// when the specified value in seconds is reached
    ///
    /// * the ban time is dynamically calculated based on the current ban list collected
    /// * tip: increase this value when using I2P features
    #[arg(long, default_value_t = 60)]
    pub timeout: u64,
}
