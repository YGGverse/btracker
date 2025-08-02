use anyhow::{Result, bail};
use std::{fs, path::PathBuf};

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

    pub fn clear_output_folder(&self, info_hash: &str) -> Result<()> {
        let mut p = PathBuf::from(&self.directory);
        p.push(info_hash);
        fs::remove_dir_all(&p)?;
        Ok(())
    }

    pub fn output_folder(&self, info_hash: &str) -> Result<String> {
        let mut p = PathBuf::from(&self.directory);
        p.push(info_hash);
        fs::create_dir(&p)?;
        Ok(p.to_string_lossy().to_string())
    }

    pub fn root(&self) -> PathBuf {
        self.directory.clone()
    }

    pub fn bytes(&self, path: &PathBuf) -> Result<Vec<u8>> {
        Ok(std::fs::read({
            let mut p = PathBuf::from(&self.directory);
            p.push(path);
            p
        })?)
    }
}
