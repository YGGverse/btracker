use serde::Deserialize;
use serde_inline_default::serde_inline_default;

const ADD_TORRENT_SECONDS: u64 = 60;
const TORRENT_PRELOAD_SECONDS: u64 = 60;
const CLEANUP_INACTIVE_I2P_SESSION_SECONDS: u64 = 900;

#[serde_inline_default]
#[derive(Deserialize)]
pub struct Timeout {
    #[serde_inline_default(ADD_TORRENT_SECONDS)]
    pub add_torrent_seconds: u64,

    #[serde_inline_default(TORRENT_PRELOAD_SECONDS)]
    pub torrent_preload_seconds: u64,

    #[serde_inline_default(CLEANUP_INACTIVE_I2P_SESSION_SECONDS)]
    pub cleanup_inactive_i2p_session_seconds: u64,
}

impl Default for Timeout {
    fn default() -> Self {
        Self {
            add_torrent_seconds: ADD_TORRENT_SECONDS,
            torrent_preload_seconds: TORRENT_PRELOAD_SECONDS,
            cleanup_inactive_i2p_session_seconds: CLEANUP_INACTIVE_I2P_SESSION_SECONDS,
        }
    }
}
