use chrono::{DateTime, Utc};
use std::{fs, io::Error, path::PathBuf, time::SystemTime};

const EXTENSION: &str = "torrent";

struct File {
    modified: SystemTime,
    path: PathBuf,
}

#[derive(Clone, Debug, Default)]
pub enum Sort {
    #[default]
    Modified,
}

#[derive(Clone, Debug, Default)]
pub enum Order {
    #[default]
    Asc,
    Desc,
}

pub struct Torrent {
    pub bytes: Vec<u8>,
    pub time: DateTime<Utc>,
}

pub struct Public {
    default_capacity: usize,
    pub default_limit: usize,
    root: PathBuf,
}

impl Public {
    // Constructors

    pub fn init(
        root: PathBuf,
        default_limit: usize,
        default_capacity: usize,
    ) -> Result<Self, String> {
        if !root.is_dir() {
            return Err("Public root is not directory".into());
        }
        Ok(Self {
            default_capacity,
            default_limit,
            root: root.canonicalize().map_err(|e| e.to_string())?,
        })
    }

    // Getters

    pub fn torrent(&self, info_hash: librqbit_core::Id20) -> Option<Torrent> {
        let mut p = PathBuf::from(&self.root);
        p.push(format!("{}.{EXTENSION}", info_hash.as_string()));
        Some(Torrent {
            bytes: fs::read(&p).ok()?,
            time: p.metadata().ok()?.modified().ok()?.into(),
        })
    }

    pub fn torrents(
        &self,
        keyword: Option<&str>,
        sort_order: Option<(Sort, Order)>,
        start: Option<usize>,
        limit: Option<usize>,
    ) -> Result<(usize, Vec<Torrent>), Error> {
        let f = self.files(keyword, sort_order)?;
        let t = f.len();
        let l = limit.unwrap_or(t);
        let mut b = Vec::with_capacity(l);
        for file in f.into_iter().skip(start.unwrap_or_default()).take(l) {
            b.push(Torrent {
                bytes: fs::read(file.path)?,
                time: file.modified.into(),
            })
        }
        Ok((t, b))
    }

    pub fn href(&self, info_hash: &str, path: &str) -> Option<String> {
        let mut relative = PathBuf::from(info_hash);
        relative.push(path);

        let mut absolute = PathBuf::from(&self.root);
        absolute.push(&relative);

        let c = absolute.canonicalize().ok()?;
        if c.starts_with(&self.root) && c.exists() {
            Some(relative.to_string_lossy().into())
        } else {
            None
        }
    }

    // Helpers

    fn files(
        &self,
        keyword: Option<&str>,
        sort_order: Option<(Sort, Order)>,
    ) -> Result<Vec<File>, Error> {
        let mut files = Vec::with_capacity(self.default_capacity);
        for dir_entry in fs::read_dir(&self.root)? {
            let entry = dir_entry?;
            let path = entry.path();
            if !path.is_file() || path.extension().is_none_or(|e| e != EXTENSION) {
                continue;
            }
            if let Some(k) = keyword
                && !k.is_empty()
                && !librqbit_core::torrent_metainfo::torrent_from_bytes(&fs::read(&path)?)
                    .is_ok_and(|m: librqbit_core::torrent_metainfo::TorrentMetaV1Owned| {
                        m.info_hash.as_string().contains(k)
                            || m.info.name.is_some_and(|n| n.to_string().contains(k))
                            || m.info.files.is_some_and(|f| {
                                f.iter().any(|f| {
                                    let mut p = PathBuf::new();
                                    f.full_path(&mut p)
                                        .is_ok_and(|_| p.to_string_lossy().contains(k))
                                })
                            })
                    })
            {
                continue;
            }
            files.push(File {
                modified: entry.metadata()?.modified()?,
                path,
            })
        }
        if let Some((sort, order)) = sort_order {
            match sort {
                Sort::Modified => match order {
                    Order::Asc => files.sort_by(|a, b| a.modified.cmp(&b.modified)),
                    Order::Desc => files.sort_by(|a, b| b.modified.cmp(&a.modified)),
                },
            }
        }
        Ok(files)
    }
}
