use clap::Parser;
use std::{
    net::{Ipv4Addr, SocketAddr, SocketAddrV4},
    path::PathBuf,
};
use url::Url;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Config {
    /// Server name
    #[arg(short, long, default_value_t = String::from("βtracker"))]
    pub name: String,

    /// Server description
    #[arg(short, long)]
    pub description: Option<String>,

    /// Date format
    #[arg(short, long, default_value_t = String::from("%Y/%m/%d"))]
    pub format_date: String,

    /// Tracker(s) to public announce
    #[arg(short, long)]
    pub tracker: Option<Vec<Url>>,

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

    /// Bind server `host:port` to listen incoming connections on it
    #[arg(short, long, default_value_t = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 1965)))]
    pub bind: SocketAddr,

    /// Filepath to server identity in PKCS (PFX) format
    #[arg(short, long)]
    pub identity: PathBuf,

    /// Passphrase to unlock encrypted identity
    #[arg(short, long, default_value_t = String::new())]
    pub password: String,

    /// btracker-fs directory
    #[arg(short = 'S', long)]
    pub storage: PathBuf,

    /// Listing items limit
    #[arg(short, long, default_value_t = 10)]
    pub limit: usize,

    /// Default index capacity
    #[arg(short, long, default_value_t = 1000)]
    pub capacity: usize,
}
