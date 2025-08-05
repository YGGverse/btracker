use clap::Parser;
use std::{
    net::{IpAddr, Ipv4Addr},
    path::PathBuf,
};
use url::Url;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Config {
    /// Path to the [aquatic-crawler](https://github.com/YGGverse/aquatic-crawler) file storage
    #[arg(long, short)]
    pub storage: PathBuf,

    /// Default listing limit
    #[arg(long, default_value_t = 50)]
    pub list_limit: usize,

    /// Default capacity (estimated torrents in `storage`)
    #[arg(long, default_value_t = 1000)]
    pub capacity: usize,

    /// Server name
    #[arg(long, default_value_t = String::from("βtracker"))]
    pub title: String,

    /// Server description
    #[arg(long)]
    pub description: Option<String>,

    /// Canonical URL
    #[arg(long)]
    pub link: Option<Url>,

    /// Display following tracker(s) in the header, append also to the magnet links
    #[arg(long)]
    pub tracker: Option<Vec<Url>>,

    /// Format timestamps (on the web view)
    ///
    /// * tip: escape with `%%d/%%m/%%Y %%H:%%M` in the CLI/bash argument
    #[arg(long, short, default_value_t = String::from("%d/%m/%Y %H:%M"))]
    pub format_time: String,

    /// Bind server on given host
    #[arg(long, short, default_value_t = IpAddr::V4(Ipv4Addr::LOCALHOST))]
    pub address: IpAddr,

    /// Bind server on given port
    #[arg(long, short, default_value_t = 8000)]
    pub port: u16,
}
