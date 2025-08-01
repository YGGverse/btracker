use anyhow::{Result, bail};
use std::path::PathBuf;

/// Temporary file storage for `librqbit` preload data
pub struct Preload {
    directory: PathBuf,
    pub max_filecount: Option<usize>,
    pub max_filesize: Option<u64>,
}

impl Preload {
    pub fn init(
        directory: PathBuf,
        max_filecount: Option<usize>,
        max_filesize: Option<u64>,
    ) -> Result<Self> {
        if !directory.is_dir() {
            bail!("Preload location is not directory!");
        }
        Ok(Self {
            max_filecount,
            max_filesize,
            directory,
        })
    }

    pub fn clear(&mut self) {
        self.directory.clear()
    }

    pub fn directory(&self) -> &PathBuf {
        &self.directory
    }

    pub fn bytes(&self, path: &PathBuf) -> Result<Vec<u8>> {
        Ok(std::fs::read({
            let mut p = PathBuf::from(&self.directory);
            p.push(path);
            p
        })?)
    }
}
