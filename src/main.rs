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
use storage::{Order, Sort, Storage};
use url::Url;

#[derive(Clone, Debug, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct Meta {
    pub canonical: Option<Url>,
    pub description: Option<String>,
    pub stats: Option<Url>,
    pub title: String,
    pub trackers: Option<Vec<Url>>,
}

#[get("/")]
fn index(storage: &State<Storage>, meta: &State<Meta>) -> Result<Template, Custom<String>> {
    Ok(Template::render(
        "index",
        context! {
            meta: meta.inner(),
            torrents: storage
            .torrents(
                Some((Sort::Modified, Order::Asc)),
                Some(storage.default_limit),
            )
            .map_err(|e| Custom(Status::InternalServerError, e.to_string()))?
        },
    ))
}

#[get("/rss")]
fn rss(feed: &State<Feed>, storage: &State<Storage>) -> Result<RawXml<String>, Custom<String>> {
    let mut b = feed.transaction(1024); // @TODO
    for torrent in storage
        .torrents(
            Some((Sort::Modified, Order::Asc)),
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
        config.tracker.clone().map(|u| u.into_iter().collect()), // make sure it's unique
    );
    let storage = Storage::init(config.storage, config.limit, config.capacity).unwrap(); // @TODO handle
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
            stats: config.stats,
            title: config.title,
            trackers: config.tracker,
        })
        .mount("/", routes![index, rss])
}
