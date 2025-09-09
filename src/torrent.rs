mod file;

use chrono::{DateTime, Utc};
use file::File;
use librqbit_core::{
    Id20,
    torrent_metainfo::{self, TorrentMetaV1Owned},
};
use rocket::serde::Serialize;

#[derive(Clone, Debug, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct Torrent {
    pub announce: Option<String>,
    pub comment: Option<String>,
    pub created_by: Option<String>,
    pub creation_date: Option<DateTime<Utc>>,
    pub files: Option<Vec<File>>,
    pub id: Id20,
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
    pub fn from_public(bytes: &[u8], time: DateTime<Utc>) -> Result<Self, String> {
        let i: TorrentMetaV1Owned =
            torrent_metainfo::torrent_from_bytes(bytes).map_err(|e| e.to_string())?;
        Ok(Torrent {
            id: i.info_hash,
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

    pub fn files(&self) -> Option<usize> {
        self.files.as_ref().map(|f| f.len())
    }

    pub fn magnet(&self, trackers: Option<&Vec<url::Url>>) -> String {
        let mut b = format!("magnet:?xt=urn:btih:{}", self.info_hash);
        if let Some(ref n) = self.name {
            b.push_str("&dn=");
            b.push_str(&urlencoding::encode(n))
        }
        if let Some(t) = trackers {
            for tracker in t {
                b.push_str("&tr=");
                b.push_str(&urlencoding::encode(tracker.as_str()))
            }
        }
        b
    }
}
