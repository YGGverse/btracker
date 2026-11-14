mod announce;
mod scrape;

use serde::Deserialize;

#[serde_inline_default::serde_inline_default]
#[derive(Deserialize)]
pub struct Tracker {
    /// HTTP info-hash source
    /// * tip: prefer localhost trackers
    #[serde(default)]
    pub scrape: Vec<scrape::Http>,

    /// HTTP peers source (CPU expensive)
    #[serde(default)]
    pub announce_http: Vec<announce::Http>,

    /// UDP peers source
    /// * tip: prefer localhost UDP trackers
    #[serde(default)]
    pub announce_udp: Vec<announce::Udp>,

    /// Separated logic for the I2P / HTTP trackers
    pub announce_i2p: Option<Vec<announce::I2p>>,
}
