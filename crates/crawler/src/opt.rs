use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Opt {
    /// Path to `config.toml`
    #[arg(long, short)]
    pub config: PathBuf,
}
