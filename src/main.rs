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
};
use storage::{Order, Sort, Storage};

#[get("/")]
pub fn index() -> &'static str {
    "Catalog in development, use /rss"
}

#[get("/rss")]
pub fn rss(feed: &State<Feed>, storage: &State<Storage>) -> Result<RawXml<String>, Custom<String>> {
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
        config.title,
        config.description,
        config.link,
        config.tracker.map(|u| u.into_iter().collect()), // make sure it's unique
    );
    let storage = Storage::init(config.storage, config.limit, config.capacity).unwrap(); // @TODO handle
    rocket::build()
        .configure(rocket::Config {
            port: config.port,
            address: config.address,
            ..rocket::Config::debug_default()
        })
        .manage(feed)
        .manage(storage)
        .mount("/", routes![index, rss])
}
