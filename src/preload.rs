use anyhow::{Result, bail};
use std::{fs, path::PathBuf};

/// Temporary file storage for `librqbit` preload data
pub struct Preload {
    root: PathBuf,
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
            root: directory.canonicalize()?,
        })
    }

    pub fn clear_output_folder(&self, info_hash: &str) -> Result<()> {
        Ok(fs::remove_dir_all(&self.path(&PathBuf::from(info_hash))?)?)
    }

    /// * create new directory if not exists
    pub fn output_folder(&self, info_hash: &str) -> Result<PathBuf> {
        let p = self.path(&PathBuf::from(info_hash))?;
        if !p.exists() {
            fs::create_dir(&p)?
        }
        Ok(p)
    }

    pub fn root(&self) -> PathBuf {
        self.root.clone()
    }

    pub fn bytes(&self, relative: &PathBuf) -> Result<Vec<u8>> {
        Ok(std::fs::read(self.path(relative)?)?)
    }

    fn path(&self, relative: &PathBuf) -> Result<PathBuf> {
        let mut p = PathBuf::from(&self.root);
        p.push(relative);
        if !p.canonicalize()?.starts_with(&self.root) {
            bail!(
                "Unexpected absolute path resolved for `{}`!",
                p.to_string_lossy()
            )
        }
        Ok(p)
    }
}
