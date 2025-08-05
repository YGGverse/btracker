use chrono::{DateTime, Utc};
use librqbit_core::{torrent_metainfo, torrent_metainfo::TorrentMetaV1Owned};
use rocket::serde::Serialize;
use std::{
    fs::{self, DirEntry},
    path::PathBuf,
};

#[derive(Clone, Debug, Default)]
pub enum Sort {
    #[default]
    Modified,
}

#[derive(Clone, Debug, Default)]
pub enum Order {
    #[default]
    Asc,
    //Desc,
}

#[derive(Clone, Debug, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct File {
    pub name: Option<String>,
    pub length: u64,
}

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

pub struct Storage {
    pub default_limit: usize,
    default_capacity: usize,
    root: PathBuf,
}

impl Storage {
    // Constructors

    pub fn init(
        root: PathBuf,
        default_limit: usize,
        default_capacity: usize,
    ) -> Result<Self, String> {
        if !root.is_dir() {
            return Err("Storage root is not directory".into());
        }
        Ok(Self {
            default_limit,
            default_capacity,
            root: root.canonicalize().map_err(|e| e.to_string())?,
        })
    }

    // Getters

    pub fn torrents(
        &self,
        sort_order: Option<(Sort, Order)>,
        start: Option<usize>,
        limit: Option<usize>,
    ) -> Result<(usize, Vec<Torrent>), String> {
        let f = self.files(sort_order)?;
        let t = f.len();
        let l = limit.unwrap_or(t);
        let mut b = Vec::with_capacity(l);
        for file in f.into_iter().skip(start.unwrap_or_default()).take(l) {
            if file
                .path()
                .extension()
                .is_none_or(|e| e.is_empty() || e.to_string_lossy() != "torrent")
            {
                return Err("Unexpected file extension".into());
            }
            let i: TorrentMetaV1Owned = torrent_metainfo::torrent_from_bytes(
                &fs::read(file.path()).map_err(|e| e.to_string())?,
            )
            .map_err(|e| e.to_string())?;

            b.push(Torrent {
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
                    let limit = 1000; // @TODO
                    let mut b = Vec::with_capacity(files.len());
                    let mut i = files.iter();
                    let mut t = 0;
                    for f in i.by_ref() {
                        if t < limit {
                            t += 1;
                            b.push(File {
                                name: String::from_utf8(
                                    f.path
                                        .iter()
                                        .enumerate()
                                        .flat_map(|(n, b)| {
                                            if n == 0 {
                                                b.0.to_vec()
                                            } else {
                                                let mut p = vec![b'/'];
                                                p.extend(b.0.to_vec());
                                                p
                                            }
                                        })
                                        .collect(),
                                )
                                .ok(),
                                length: f.length,
                            });
                            continue;
                        }
                        // limit reached: count sizes left and use placeholder as the last item name
                        let mut l = 0;
                        for f in i.by_ref() {
                            l += f.length
                        }
                        b.push(File {
                            name: Some("...".to_string()),
                            length: l,
                        });
                        break;
                    }
                    b[..t].sort_by(|a, b| a.name.cmp(&b.name)); // @TODO optional
                    b
                }),
                publisher_url: i.publisher_url.map(|u| u.to_string()),
                publisher: i.publisher.map(|p| p.to_string()),
                is_private: i.info.private,
                length: i.info.length,
                name: i.info.name.map(|e| e.to_string()),
                time: file
                    .metadata()
                    .map_err(|e| e.to_string())?
                    .modified()
                    .map_err(|e| e.to_string())?
                    .into(),
            })
        }
        Ok((t, b))
    }

    // Helpers

    fn files(&self, sort_order: Option<(Sort, Order)>) -> Result<Vec<DirEntry>, String> {
        let mut b = Vec::with_capacity(self.default_capacity);
        for entry in fs::read_dir(&self.root).map_err(|e| e.to_string())? {
            let e = entry.map_err(|e| e.to_string())?;
            match e.file_type() {
                Ok(t) => {
                    if t.is_file() {
                        b.push((e.metadata().unwrap().modified().unwrap(), e))
                    }
                }
                Err(e) => warn!("{}", e.to_string()),
            }
        }
        if let Some((sort, order)) = sort_order {
            match sort {
                Sort::Modified => match order {
                    Order::Asc => b.sort_by(|a, b| a.0.cmp(&b.0)),
                    //Order::Desc => b.sort_by(|a, b| b.0.cmp(&a.0)),
                },
            }
        }
        Ok(b.into_iter().map(|e| e.1).collect())
    }
}
