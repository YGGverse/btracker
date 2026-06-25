mod announce;
mod scrape;

use announce::{Default, I2p};
use scrape::Scrape;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct Tracker {
    /// Info-hash source
    pub scrape: Vec<Scrape>,

    /// Peers source
    pub announce: Vec<Default>,
    pub announce_i2p: Option<Vec<I2p>>,
}
