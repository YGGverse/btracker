use serde::Deserialize;
use serde_inline_default::serde_inline_default;

#[serde_inline_default]
#[derive(Deserialize)]
pub struct Timeout {
    #[serde_inline_default(60)]
    pub add_torrent_seconds: u64,

    #[serde_inline_default(60)]
    pub torrent_preload_seconds: u64,

    #[serde_inline_default(900)]
    pub cleanup_inactive_i2p_session_seconds: u64,
}

impl Default for Timeout {
    fn default() -> Self {
        Self {
            add_torrent_seconds: 60,
            torrent_preload_seconds: 60,
            cleanup_inactive_i2p_session_seconds: 900,
        }
    }
}
