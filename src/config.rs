use clap::Parser;
use std::{
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6},
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
    /// * torrents with files collected by the [aquatic-crawler](https://github.com/yggverse/aquatic-crawler)
    #[arg(long)]
    pub public: PathBuf,

    /// Server name
    #[arg(long, default_value_t = String::from("βtracker"))]
    pub title: String,

    /// Server description
    #[arg(long)]
    pub description: Option<String>,

    /// Canonical URL
    #[arg(long)]
    pub canonical_url: Option<Url>,

    /// Display following tracker(s) in the header, append also to the magnet links
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

    /// Bind local UDP socket on given address
    ///
    /// * the default UDP server is not in use without the optional `scrape` argument value
    #[arg(long, default_values_t = vec![
        SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 0)),
        SocketAddr::V6(SocketAddrV6::new(Ipv6Addr::UNSPECIFIED, 0, 0, 0))
    ])]
    pub udp: Vec<SocketAddr>,

    /// Scrape given trackers (to display peers/seeders/leechers info)
    ///
    /// * supports multi-stack IPv4/IPv6 trackers
    #[arg(long)]
    pub scrape: Option<Vec<Url>>,

    /// Configure instance in the debug mode
    #[arg(long, default_value_t = false)]
    pub debug: bool,
}
