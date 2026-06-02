use btracker_fs::public::Storage;
use librqbit_core::Id20;
use regex::Regex;
use std::{path::PathBuf, str::FromStr};
use url::Url;

pub enum Route {
    File(PathBuf),
    Info(Id20),
    List {
        keyword: Option<String>,
        page: Option<usize>,
    },
    NotFound,
    Search,
}

impl Route {
    pub fn from_url(url: &Url, public: &Storage) -> Self {
        let p = urlencoding::decode(url.path()).ok().unwrap_or_default();
        let t = p.trim_matches('/');
        let q = url.query();

        if p.is_empty() {
            return Self::List {
                keyword: None,
                page: None,
            };
        }

        if let Some(path) = public.filepath(t) {
            return Self::File(path);
        }

        if let Ok(id) = Id20::from_str(t) {
            return Self::Info(id);
        }

        if p == "/search" && q.is_none() {
            return Self::Search;
        }

        if Regex::new(r"^/(|search)").unwrap().is_match(&p) {
            return Self::List {
                keyword: q.and_then(|k| urlencoding::decode(k).ok().map(|k| k.into())),
                page: Regex::new(r"/(\d+)$").unwrap().captures(&p).map(|c| {
                    c.get(1)
                        .map_or(1, |p| p.as_str().parse::<usize>().unwrap_or(1))
                }),
            };
        }

        Self::NotFound
    }
}
