use serde::Deserialize;
use serde_inline_default::serde_inline_default;
use url::Url;

/// Info-hash source
/// * tip: by using OpenTracker,
///    make sure `FEATURES+=-DWANT_FULLSCRAPE` is enabled!
#[serde_inline_default]
#[derive(Deserialize)]
pub struct Scrape {
    /// URL to the BEP 48 / Full Scrape
    ///
    /// * supports HTTP trackers only
    pub url: Url,

    /// How long to wait for tracker full scrape response
    #[serde_inline_default(5)]
    pub timeout: u64,

    /// Use HTTP(s) proxy, e.g. `http://127.0.0.1:9050` or `http://127.0.0.1:4444` for I2P
    pub proxy: Option<Url>,
}
