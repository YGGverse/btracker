use clap::Parser;
use std::{
    net::{Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6},
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

    /// Tracker(s) to join / scrape requests
    #[arg(short, long)]
    pub tracker: Option<Vec<Url>>,

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
    #[arg(short, long)]
    pub storage: PathBuf,

    /// Listing items limit
    #[arg(short, long, default_value_t = 10)]
    pub limit: usize,

    /// Default index capacity
    #[arg(short, long, default_value_t = 1000)]
    pub capacity: usize,

    /// Bind scrape UDP server
    ///
    /// * requires `tracker` value(s) to enable scrape features
    #[arg(long, default_values_t = vec![
        SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 0)),
        SocketAddr::V6(SocketAddrV6::new(Ipv6Addr::UNSPECIFIED, 0, 0, 0))
    ])]
    pub udp: Vec<SocketAddr>,
}
