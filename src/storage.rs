use chrono::{DateTime, Utc};
use std::{
    fs::{self, DirEntry},
    io::Error,
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
    Desc,
}

pub struct Torrent {
    pub bytes: Vec<u8>,
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
                    .is_some_and(|e| !e.is_empty() && e.to_string_lossy() == "torrent")
            })
        {
            b.push(Torrent {
                bytes: fs::read(file.path())?,
                time: file.metadata()?.modified()?.into(),
            })
        }
        Ok((t, b))
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
