use crate::{Meta, Scrape, Scraper, Torrent};
use rocket::{State, serde::Serialize};

#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
pub struct Format {
    pub created: Option<String>,
    pub files: String,
    pub indexed: String,
    pub magnet: String,
    pub scrape: Option<Scrape>,
    pub size: String,
    pub torrent: Torrent,
}

impl Format {
    pub fn from_torrent(torrent: Torrent, scraper: &State<Scraper>, meta: &State<Meta>) -> Self {
        Self {
            created: torrent
                .creation_date
                .map(|t| t.format(&meta.format_time).to_string()),
            indexed: torrent.time.format(&meta.format_time).to_string(),
            magnet: torrent.magnet(meta.trackers.as_ref()),
            scrape: scraper.scrape(&torrent.info_hash),
            size: torrent.size(),
            files: torrent.files(),
            torrent,
        }
    }
}
