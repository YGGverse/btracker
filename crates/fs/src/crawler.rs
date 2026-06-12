use regex::Regex;
use std::{collections::HashSet, fs, io::Error, path::PathBuf};

pub struct Storage {
    root: PathBuf,
    pub max_filecount: Option<usize>,
    pub max_filesize: Option<u64>,
    pub regex: Option<Regex>,
}

impl Storage {
    // Constructors

    pub fn init(
        root: PathBuf,
        regex: Option<Regex>,
        max_filecount: Option<usize>,
        max_filesize: Option<u64>,
    ) -> Result<Self, String> {
        // make sure given path is valid and exist
        if !root.is_dir() {
            return Err("Storage root is not directory".into());
        }
        Ok(Self {
            max_filecount,
            max_filesize,
            regex,
            root: root.canonicalize().map_err(|e| e.to_string())?,
        })
    }

    // Actions

    /// Persist torrent bytes and preloaded content,
    /// cleanup tmp data on success (see rqbit#408)
    pub fn commit(
        &self,
        info_hash: &str,
        torrent_bytes: Vec<u8>,
        persist_files: Option<HashSet<PathBuf>>,
    ) -> Result<(), Error> {
        // persist preloaded files
        let permanent_dir = self.permanent_dir(info_hash, true)?;
        // init temporary path without creating the dir (delegate to `librqbit`)
        let tmp_dir = self.tmp_dir(info_hash, false)?;
        if let Some(files) = persist_files {
            let components_count = permanent_dir.components().count(); // count root offset once
            for file in files {
                // build the absolute path for the relative torrent filename
                let tmp_file = {
                    let mut p = PathBuf::from(&tmp_dir);
                    p.push(file);
                    p.canonicalize()?
                };
                // make sure preload path is referring to the expected location
                assert!(tmp_file.starts_with(&self.root) && !tmp_file.is_dir());
                // build new permanent path /root/info-hash
                let mut permanent_file = PathBuf::from(&permanent_dir);
                for component in tmp_file.components().skip(components_count) {
                    permanent_file.push(component)
                }
                // make sure segments count is same to continue
                assert!(tmp_file.components().count() == permanent_file.components().count());
                // move `persist_files` from temporary to permanent location
                fs::create_dir_all(permanent_file.parent().unwrap())?;
                fs::rename(&tmp_file, &permanent_file)?;
                log::debug!(
                    "persist tmp file `{}` to `{}`",
                    tmp_file.to_string_lossy(),
                    permanent_file.to_string_lossy()
                );
            }
        }
        // cleanup temporary data
        if tmp_dir.exists() {
            fs::remove_dir_all(&tmp_dir)?;
            log::debug!("clean tmp data `{}`", tmp_dir.to_string_lossy())
        }
        // persist torrent bytes to file (on previous operations success)
        let torrent_file = self.torrent(info_hash);
        fs::write(&torrent_file, torrent_bytes)?;
        log::debug!(
            "persist torrent bytes for `{}`",
            torrent_file.to_string_lossy()
        );
        Ok(())
    }

    // Actions

    /// Build the absolute path to the temporary directory
    /// * optionally creates directory if not exists
    pub fn tmp_dir(&self, info_hash: &str, is_create: bool) -> Result<PathBuf, Error> {
        let mut p = PathBuf::from(&self.root);
        p.push(tmp_component(info_hash));
        assert!(!p.is_file());
        if is_create && !p.exists() {
            fs::create_dir(&p)?;
            log::debug!("create tmp directory `{}`", p.to_string_lossy())
        }
        Ok(p)
    }

    /// Build the absolute path to the permanent directory
    /// * optionally removes directory with its content
    fn permanent_dir(&self, info_hash: &str, is_clear: bool) -> Result<PathBuf, Error> {
        let mut p = PathBuf::from(&self.root);
        p.push(info_hash);
        assert!(!p.is_file());
        if is_clear && p.exists() {
            // clean previous data
            fs::remove_dir_all(&p)?;
            log::debug!("clean previous data `{}`", p.to_string_lossy())
        }
        Ok(p)
    }

    // Getters

    /// Get root location for `Self`
    pub fn root(&self) -> &PathBuf {
        &self.root
    }

    /// Check the given hash is contain resolved torrent file
    pub fn contains_torrent(&self, info_hash: &str) -> Result<bool, Error> {
        fs::exists(self.torrent(info_hash))
    }

    /// Get absolute path to the torrent file
    fn torrent(&self, info_hash: &str) -> PathBuf {
        let mut p = PathBuf::from(&self.root);
        p.push(format!("{info_hash}.torrent"));
        assert!(!p.is_dir());
        p
    }
}

/// Build constant path component
fn tmp_component(info_hash: &str) -> String {
    format!(".{info_hash}")
}
