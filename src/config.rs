use clap::Parser;
use std::path::PathBuf;
use url::Url;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Config {
    /// Path to the [aquatic-crawler](https://github.com/YGGverse/aquatic-crawler) file storage
    #[arg(long, short)]
    pub storage: PathBuf,

    /// Default listing limit
    #[arg(long, default_value_t = 50)]
    pub limit: usize,

    /// Default capacity (estimated torrents in `storage`)
    #[arg(long, default_value_t = 1000)]
    pub capacity: usize,

    /// Server name
    #[arg(long, default_value_t = String::from("YGGtracker"))]
    pub title: String,

    /// Server description
    #[arg(long)]
    pub description: Option<String>,

    /// Canonical URL
    #[arg(long)]
    pub link: Option<Url>,

    /// Appends following tracker(s) to the magnet links
    #[arg(long)]
    pub tracker: Option<Vec<Url>>,
}
