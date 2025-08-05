#[macro_use]
extern crate rocket;

mod config;
mod feed;
mod format;
mod storage;

use config::Config;
use feed::Feed;
use rocket::{
    State,
    http::Status,
    response::{content::RawXml, status::Custom},
    serde::Serialize,
};
use rocket_dyn_templates::{Template, context};
use std::collections::HashSet;
use storage::{Order, Sort, Storage, Torrent};
use url::Url;

#[derive(Clone, Debug, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct Meta {
    pub canonical: Option<Url>,
    pub description: Option<String>,
    pub format_time: String,
    pub stats: Option<Url>,
    pub title: String,
    pub trackers: Option<HashSet<Url>>,
}

#[get("/?<page>")]
fn index(
    page: Option<usize>,
    storage: &State<Storage>,
    meta: &State<Meta>,
) -> Result<Template, Custom<String>> {
    #[derive(Serialize)]
    #[serde(crate = "rocket::serde")]
    struct Row {
        created: Option<String>,
        indexed: String,
        magnet: String,
        size: String,
        torrent: Torrent,
    }
    let rows = storage
        .torrents(
            Some((Sort::Modified, Order::Asc)),
            page.map(|p| if p > 0 { p - 1 } else { p } * storage.default_limit),
            Some(storage.default_limit),
        )
        .map_err(|e| Custom(Status::InternalServerError, e.to_string()))?
        .into_iter()
        .map(|torrent| Row {
            created: torrent
                .creation_date
                .map(|t| t.format(&meta.format_time).to_string()),
            indexed: torrent.time.format(&meta.format_time).to_string(),
            magnet: format::magnet(&torrent.info_hash, meta.trackers.as_ref()),
            size: format::bytes(torrent.size),
            torrent,
        })
        .collect::<Vec<Row>>();
    Ok(Template::render(
        "index",
        context! {
            meta: meta.inner(),
            back: page.map(|p| uri!(index(if p > 2 { Some(p - 1) } else { None }))),
            next: if rows.len() < storage.default_limit { None }
                    else { Some(uri!(index(Some(page.map_or(2, |p| p + 1))))) },
            rows
        },
    ))
}

#[get("/rss")]
fn rss(feed: &State<Feed>, storage: &State<Storage>) -> Result<RawXml<String>, Custom<String>> {
    let mut b = feed.transaction(1024); // @TODO
    for torrent in storage
        .torrents(
            Some((Sort::Modified, Order::Asc)),
            None,
            Some(storage.default_limit),
        )
        .map_err(|e| Custom(Status::InternalServerError, e.to_string()))?
    {
        feed.push(&mut b, torrent)
    }
    Ok(RawXml(feed.commit(b)))
}

#[launch]
fn rocket() -> _ {
    use clap::Parser;
    let config = Config::parse();
    let feed = Feed::init(
        config.title.clone(),
        config.description.clone(),
        config.link.clone(),
        config.tracker.clone().map(|u| u.into_iter().collect()),
    );
    let storage = Storage::init(config.storage, config.list_limit, config.capacity).unwrap(); // @TODO handle
    rocket::build()
        .attach(Template::fairing())
        .configure(rocket::Config {
            port: config.port,
            address: config.address,
            ..rocket::Config::debug_default()
        })
        .manage(feed)
        .manage(storage)
        .manage(Meta {
            canonical: config.link,
            description: config.description,
            format_time: config.format_time,
            stats: config.stats,
            title: config.title,
            trackers: config.tracker.map(|u| u.into_iter().collect()),
        })
        .mount("/", routes![index, rss])
}
