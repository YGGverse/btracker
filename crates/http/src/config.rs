use clap::Parser;
use std::{
    net::{IpAddr, Ipv4Addr},
    path::PathBuf,
};
use url::Url;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Config {
    /// Path to the `public` directory
    ///
    /// This location must contain:
    /// * the default or custom `/public/*` files (see the [Rocket deploying specification](https://rocket.rs/guide/v0.5/deploying/))
    /// * torrents with files collected by the `btracker-crawler`
    #[arg(long)]
    pub public: PathBuf,

    /// Server name
    /// * append also to the torrent files as a comment
    #[arg(long, default_value_t = String::from("βtracker"))]
    pub title: String,

    /// Server description
    /// * append also to the torrent files as a comment
    #[arg(long)]
    pub description: Option<String>,

    /// Canonical URL
    /// * append also to the torrent files as a comment
    #[arg(long)]
    pub canonical_url: Option<Url>,

    /// Display following tracker(s) in the header
    /// * append also to the torrent files and magnet links as announce/list
    /// * make sure that `/info_hash_v1.torrent` URI ignored by the proxy
    #[arg(long)]
    pub tracker: Option<Vec<Url>>,

    /// Format timestamps (on the web view)
    ///
    /// * tip: escape with `%%d/%%m/%%Y %%H:%%M` in the CLI/bash argument
    #[arg(long, default_value_t = String::from("%d/%m/%Y %H:%M"))]
    pub format_time: String,

    /// Default listing limit
    #[arg(long, default_value_t = 20)]
    pub list_limit: usize,

    /// Default capacity (estimated torrents in the `public` directory)
    #[arg(long, default_value_t = 1000)]
    pub capacity: usize,

    /// Bind server on given host
    #[arg(long, default_value_t = IpAddr::V4(Ipv4Addr::LOCALHOST))]
    pub host: IpAddr,

    /// Bind server on given port
    #[arg(long, short, default_value_t = 8000)]
    pub port: u16,

    /// Scrape(s) to local peers count resolve
    #[arg(long)]
    pub scrape: Vec<Url>,

    /// Timeout to wait for scrape response
    #[arg(long, default_value_t = 1)]
    pub scrape_timeout: u64,

    /// Proxy for scrape requests
    #[arg(long)]
    pub scrape_proxy: Option<Url>,

    /// Proxy for I2P `tracker` scrape requests
    #[arg(long)]
    pub scrape_proxy_i2p: Option<Url>,

    /// Configure instance in the debug mode
    #[arg(long, default_value_t = false)]
    pub debug: bool,
}
