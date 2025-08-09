use chrono::{DateTime, Utc};
use std::{
    fs::{self, DirEntry},
    io::Error,
    path::PathBuf,
};

const EXTENSION: &str = "torrent";

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
    pub default_limit: usize,
    default_capacity: usize,
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
            default_limit,
            default_capacity,
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
        sort_order: Option<(Sort, Order)>,
        start: Option<usize>,
        limit: Option<usize>,
    ) -> Result<(usize, Vec<Torrent>), Error> {
        let f = self.files(sort_order)?;
        let t = f.len();
        let l = limit.unwrap_or(t);
        let mut b = Vec::with_capacity(l);
        for file in f
            .into_iter()
            .skip(start.unwrap_or_default())
            .take(l)
            .filter(|f| {
                f.path()
                    .extension()
                    .is_some_and(|e| !e.is_empty() && e.to_string_lossy() == EXTENSION)
            })
        {
            b.push(Torrent {
                bytes: fs::read(file.path())?,
                time: file.metadata()?.modified()?.into(),
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

    fn files(&self, sort_order: Option<(Sort, Order)>) -> Result<Vec<DirEntry>, Error> {
        let mut b = Vec::with_capacity(self.default_capacity);
        for entry in fs::read_dir(&self.root)? {
            let e = entry?;
            if e.file_type()?.is_file() {
                b.push((e.metadata()?.modified()?, e))
            }
        }
        if let Some((sort, order)) = sort_order {
            match sort {
                Sort::Modified => match order {
                    Order::Asc => b.sort_by(|a, b| a.0.cmp(&b.0)),
                    Order::Desc => b.sort_by(|a, b| b.0.cmp(&a.0)),
                },
            }
        }
        Ok(b.into_iter().map(|e| e.1).collect())
    }
}
