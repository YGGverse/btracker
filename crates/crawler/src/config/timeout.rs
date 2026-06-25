use serde::Deserialize;
use serde_inline_default::serde_inline_default;

#[serde_inline_default]
#[derive(Deserialize)]
pub struct Timeout {
    #[serde_inline_default(60)]
    pub add_torrent_seconds: u64,

    #[serde_inline_default(60)]
    pub torrent_preload_seconds: u64,
}
