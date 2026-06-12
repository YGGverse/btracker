//! Frontend features for the following βtracker project components:
//!
//! * https://github.com/YGGverse/btracker
//! * https://github.com/YGGverse/btracker-gemini

use chrono::{DateTime, Utc};
use librqbit_core::Id20;
use std::{
    fs,
    future::Future,
    io::Error,
    path::{Path, PathBuf},
    str::FromStr,
    time::SystemTime,
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
    Desc,
}

pub struct Torrent {
    pub bytes: Vec<u8>,
    pub time: DateTime<Utc>,
}

pub struct Storage {
    default_capacity: usize,
    pub default_limit: usize,
    root: PathBuf,
}

impl Storage {
    // Constructors

    pub fn init(
        root: &Path,
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

    pub fn torrent(&self, info_hash: Id20) -> Option<Torrent> {
        let mut p = PathBuf::from(&self.root);
        p.push(format!("{}.{E}", info_hash.as_string()));
        Some(Torrent {
            bytes: fs::read(&p).ok()?,
            time: p.metadata().ok()?.modified().ok()?.into(),
        })
    }

    pub async fn torrents<F, Fut>(
        &self,
        keyword: Option<&str>,
        sort_order: Option<(Sort, Order)>,
        start: Option<usize>,
        limit: Option<usize>,
        visibility_filter: F,
    ) -> Result<Torrents, Error>
    where
        F: Fn(Id20) -> Fut,
        Fut: Future<Output = bool>,
    {
        let f = self.files(keyword, sort_order)?;
        let t = f.len(); // total
        let l = limit.unwrap_or(t);
        let s = start.unwrap_or_default();
        let mut b = Vec::with_capacity(l);
        let mut i = 0; // start offset
        for file in f.iter() {
            if let Some(n) = file.path.file_stem()
                && let Ok(id20) = Id20::from_str(&n.to_string_lossy())
                && visibility_filter(id20).await
            {
                if i >= s && b.len() < l {
                    b.push(Torrent {
                        bytes: fs::read(&file.path)?,
                        time: file.modified.into(),
                    });
                }
                i += 1;
            }
        }
        Ok(Torrents {
            total: t,
            visible: i,
            list: b,
        })
    }

    /// Build URI for given `path`
    ///
    /// * result requires URL encode
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

    /// Return canonical absolute path to file
    ///
    /// * `None` if the given URI does not exist or has denied location
    pub fn filepath(&self, relative: &str) -> Option<PathBuf> {
        let mut p = PathBuf::from(&self.root);
        p.push(relative);

        let c = p.canonicalize().ok()?;
        if c.starts_with(&self.root) && c.is_file() {
            Some(c)
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
            if !path.is_file() || path.extension().is_none_or(|e| e != E) {
                continue;
            }
            if let Some(k) = keyword
                && !k.trim_matches(S).is_empty()
                && !librqbit_core::torrent_metainfo::torrent_from_bytes(&fs::read(&path)?)
                    .is_ok_and(|m: librqbit_core::torrent_metainfo::TorrentMetaV1Owned| {
                        k.split(S)
                            .filter(|s| !s.is_empty())
                            .map(|s| s.trim().to_lowercase())
                            .all(|q| {
                                m.info_hash.as_string().to_lowercase().contains(&q)
                                    || m.info
                                        .name
                                        .as_ref()
                                        .is_some_and(|n| n.to_string().to_lowercase().contains(&q))
                                    || m.comment
                                        .as_ref()
                                        .is_some_and(|c| c.to_string().to_lowercase().contains(&q))
                                    || m.created_by
                                        .as_ref()
                                        .is_some_and(|c| c.to_string().to_lowercase().contains(&q))
                                    || m.publisher
                                        .as_ref()
                                        .is_some_and(|p| p.to_string().to_lowercase().contains(&q))
                                    || m.publisher_url
                                        .as_ref()
                                        .is_some_and(|u| u.to_string().to_lowercase().contains(&q))
                                    || m.announce
                                        .as_ref()
                                        .is_some_and(|a| a.to_string().to_lowercase().contains(&q))
                                    || m.announce_list.iter().any(|l| {
                                        l.iter().any(|a| a.to_string().to_lowercase().contains(&q))
                                    })
                                    || m.info.files.as_ref().is_some_and(|f| {
                                        f.iter().any(|f| {
                                            let mut p = PathBuf::new();
                                            f.full_path(&mut p).is_ok_and(|_| {
                                                p.to_string_lossy().to_lowercase().contains(&q)
                                            })
                                        })
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
                    Order::Asc => files.sort_by_key(|a| a.modified),
                    Order::Desc => files.sort_by_key(|b| std::cmp::Reverse(b.modified)),
                },
            }
        }
        Ok(files)
    }
}

// Local members

/// Torrent file extension
const E: &str = "torrent";

/// Search keyword separators
const S: &[char] = &[
    '_', '-', ':', ';', ',', '(', ')', '[', ']', '/', '!', '?', ' ', // @TODO make optional
];

struct File {
    modified: SystemTime,
    path: PathBuf,
}

pub struct Torrents {
    pub total: usize,
    pub visible: usize,
    pub list: Vec<Torrent>,
}
