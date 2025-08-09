mod file;

use chrono::{DateTime, Utc};
use file::File;
use librqbit_core::{torrent_metainfo, torrent_metainfo::TorrentMetaV1Owned};
use rocket::serde::Serialize;

#[derive(Clone, Debug, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct Torrent {
    pub announce: Option<String>,
    pub comment: Option<String>,
    pub created_by: Option<String>,
    pub creation_date: Option<DateTime<Utc>>,
    pub files: Option<Vec<File>>,
    pub info_hash: String,
    pub is_private: bool,
    pub length: Option<u64>,
    pub name: Option<String>,
    pub publisher_url: Option<String>,
    pub publisher: Option<String>,
    pub size: u64,
    /// File (modified)
    pub time: DateTime<Utc>,
}

impl Torrent {
    pub fn from_storage(bytes: &[u8], time: DateTime<Utc>) -> Result<Self, String> {
        let i: TorrentMetaV1Owned =
            torrent_metainfo::torrent_from_bytes(bytes).map_err(|e| e.to_string())?;
        Ok(Torrent {
            info_hash: i.info_hash.as_string(),
            announce: i.announce.map(|a| a.to_string()),
            comment: i.comment.map(|c| c.to_string()),
            created_by: i.created_by.map(|c| c.to_string()),
            creation_date: i
                .creation_date
                .map(|t| DateTime::from_timestamp_nanos(t as i64)),
            size: i.info.length.unwrap_or_default()
                + i.info
                    .files
                    .as_ref()
                    .map(|files| files.iter().map(|f| f.length).sum::<u64>())
                    .unwrap_or_default(),
            files: i.info.files.map(|files| {
                let mut b = Vec::with_capacity(files.len());
                for f in files {
                    let mut p = std::path::PathBuf::new();
                    b.push(File {
                        length: f.length,
                        path: match f.full_path(&mut p) {
                            Ok(()) => Some(p),
                            Err(e) => {
                                warn!("Filename decode error: {e}");
                                None
                            }
                        },
                    })
                }
                b.sort_by(|a, b| a.path.cmp(&b.path)); // @TODO optional
                b
            }),
            publisher_url: i.publisher_url.map(|u| u.to_string()),
            publisher: i.publisher.map(|p| p.to_string()),
            is_private: i.info.private,
            length: i.info.length,
            name: i.info.name.map(|e| e.to_string()),
            time,
        })
    }

    // Format getters

    pub fn files(&self) -> String {
        use plurify::Plurify;
        self.files.as_ref().map_or("1 file".into(), |f| {
            let l = f.len();
            format!("{l} {}", l.plurify(&["file", "files", "files"]))
        })
    }

    pub fn size(&self) -> String {
        size(self.size)
    }

    pub fn magnet(&self, trackers: Option<&Vec<url::Url>>) -> String {
        let mut b = if self.info_hash.len() == 40 {
            format!("magnet:?xt=urn:btih:{}", self.info_hash)
        } else {
            todo!("info-hash v2 yet not supported") // librqbit_core::hash_id::Id
        };
        if let Some(t) = trackers {
            for tracker in t {
                b.push_str("&tr=");
                b.push_str(&urlencoding::encode(tracker.as_str()))
            }
        }
        b
    }
}

fn size(value: u64) -> String {
    const KB: f32 = 1024.0;
    const MB: f32 = KB * KB;
    const GB: f32 = MB * KB;

    let f = value as f32;

    if f < KB {
        format!("{value} B")
    } else if f < MB {
        format!("{:.2} KB", f / KB)
    } else if f < GB {
        format!("{:.2} MB", f / MB)
    } else {
        format!("{:.2} GB", f / GB)
    }
}
