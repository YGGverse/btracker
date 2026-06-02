use clap::Parser;
use regex::Regex;
use std::{net::SocketAddr, path::PathBuf};
use url::Url;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Config {
    /// Directory path to store preloaded data (e.g. `.torrent` files)
    ///
    /// * it's probably the same location as `public` dir for the [btracker](https://github.com/YGGverse/btracker) frontend
    #[arg(long, short)]
    pub preload: PathBuf,

    /// Absolute path(s) or URL(s) to the BEP 48 / Full Scrape
    #[arg(long, short)]
    pub full_scrape: Vec<String>,

    /// The P2P Blocklist file URL (to filter outgoing connections)
    ///
    /// * use `--blocklist=file:///path/to/blocklist.txt` format for the local path
    #[arg(long)]
    pub blocklist: Option<Url>,

    /// Define custom tracker(s) to preload the `.torrent` files info
    #[arg(long, short)]
    pub tracker: Vec<Url>,

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

    /// Preload only files match regex pattern (list only without preload by default)
    /// * see also `preload_max_filesize`, `preload_max_filecount` options
    ///
    /// ## Example:
    ///
    /// Filter by image ext
    /// ```
    /// --preload-regex '\.(png|gif|jpeg|jpg|webp|svg|log|nfo|txt)$'
    /// ```
    ///
    /// * requires `storage` argument defined
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

    /// Use `socks5://[username:password@]host:port`
    #[arg(long)]
    pub proxy_url: Option<Url>,

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
    #[arg(long, default_value_t = 60)]
    pub timeout: u64,
}
