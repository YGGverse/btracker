use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Config {
    /// Path to the permanent [redb](https://www.redb.org) database
    #[arg(long, short)]
    pub database: PathBuf,

    /// Print debug output
    #[arg(long, default_value_t = false)]
    pub debug: bool,

    /// Absolute path(s) or URL(s) to import infohashes from the Aquatic tracker binary API
    ///
    /// * PR#233 feature ([Wiki](https://github.com/YGGverse/aquatic-crawler/wiki/Aquatic))
    #[arg(long, short)]
    pub infohash: Vec<String>,

    /// Define custom tracker(s) to preload the `.torrent` files info
    #[arg(long, short)]
    pub tracker: Vec<String>,

    /// Define initial peer(s) to preload the `.torrent` files info
    #[arg(long)]
    pub initial_peer: Vec<String>,

    /// Appends `--tracker` value to magnets and torrents
    #[arg(long, default_value_t = false)]
    pub export_trackers: bool,

    /// Enable DHT resolver
    #[arg(long, default_value_t = false)]
    pub enable_dht: bool,

    /// Bind resolver session on specified device name (`tun0`, `mycelium`, etc.)
    #[arg(long)]
    pub bind: Option<String>,

    /// Directory path to store temporary preload data
    #[arg(long, short)]
    pub preload: PathBuf,

    /// Max size sum of preloaded files per torrent (match `preload_regex`)
    #[arg(long)]
    pub preload_max_filesize: Option<u64>,

    /// Max count of preloaded files per torrent (match `preload_regex`)
    #[arg(long)]
    pub preload_max_filecount: Option<usize>,

    /// Use `socks5://[username:password@]host:port`
    #[arg(long)]
    pub proxy_url: Option<String>,

    // Peer options
    #[arg(long)]
    pub peer_connect_timeout: Option<u64>,

    #[arg(long)]
    pub peer_read_write_timeout: Option<u64>,

    #[arg(long)]
    pub peer_keep_alive_interval: Option<u64>,

    /// Estimated info-hash index capacity
    #[arg(long, default_value_t = 1000)]
    pub index_capacity: usize,

    /// Max time to handle each torrent
    #[arg(long, default_value_t = 10)]
    pub add_torrent_timeout: u64,

    /// Crawl loop delay in seconds
    #[arg(long, default_value_t = 300)]
    pub sleep: u64,

    /// Limit upload speed (b/s)
    #[arg(long)]
    pub upload_limit: Option<u32>,

    /// Limit download speed (b/s)
    #[arg(long)]
    pub download_limit: Option<u32>,
}
