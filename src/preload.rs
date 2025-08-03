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
        if !is_info_hash(info_hash) {
            bail!("Invalid info-hash `{info_hash}`")
        }
        let mut p = PathBuf::from(&self.root);
        p.push(info_hash);
        if !p.is_dir() {
            bail!(
                "Requested target `{}` is not directory!",
                p.to_string_lossy()
            )
        }
        Ok(fs::remove_dir_all(&p)?)
    }

    /// * create new directory if not exists
    pub fn output_folder(&self, info_hash: &str) -> Result<PathBuf> {
        if !is_info_hash(info_hash) {
            bail!("Invalid info-hash `{info_hash}`")
        }
        let mut p = PathBuf::from(&self.root);
        p.push(info_hash);
        if !p.exists() {
            fs::create_dir(&p)?
        }
        Ok(p)
    }

    pub fn root(&self) -> &PathBuf {
        &self.root
    }

    pub fn bytes(&self, relative: &PathBuf) -> Result<Vec<u8>> {
        let mut p = PathBuf::from(&self.root);
        p.push(relative);
        // make sure that given relative path
        // does not contain relative navigation entities
        if !p.canonicalize()?.starts_with(&self.root) {
            bail!(
                "Unexpected absolute path resolved for `{}`!",
                p.to_string_lossy()
            )
        }
        Ok(std::fs::read(p)?)
    }
}

fn is_info_hash(value: &str) -> bool {
    value.len() == 40 && value.chars().all(|c| c.is_ascii_hexdigit())
}
