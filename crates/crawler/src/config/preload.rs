use serde::Deserialize;
use std::path::PathBuf;

#[derive(Deserialize)]
pub struct Preload {
    /// Directory path to store preloaded data (e.g. `.torrent` files)
    ///
    /// * it's probably the same location as `public` dir for the `btracker-http` frontend
    pub path: PathBuf,

    /// Preload content file (names) match `regex` pattern
    /// * see also `max_filesize`, `max_filesize` options
    ///
    /// ## Example:
    ///
    /// ```
    /// \.(png|gif|jpeg|jpg|webp|svg|log|nfo|txt)$
    /// ```
    pub regex: Option<String>,

    /// Max size sum of preloaded files per torrent (match `regex`)
    pub max_filesize: Option<u64>,

    /// Max count of preloaded files per torrent (match `regex`)
    pub max_filecount: Option<usize>,
}
