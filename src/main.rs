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
use storage::{Order, Sort, Storage, Torrent};
use url::Url;

#[derive(Clone, Debug, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct Meta {
    pub canonical: Option<Url>,
    pub description: Option<String>,
    pub format_time: String,
    pub title: String,
    /// * use vector to keep the order from the arguments list
    pub trackers: Option<Vec<Url>>,
}

#[get("/?<page>")]
fn index(
    page: Option<usize>,
    storage: &State<Storage>,
    meta: &State<Meta>,
) -> Result<Template, Custom<String>> {
    use plurify::Plurify;
    #[derive(Serialize)]
    #[serde(crate = "rocket::serde")]
    struct Row {
        created: Option<String>,
        indexed: String,
        magnet: String,
        size: String,
        files: String,
        torrent: Torrent,
    }
    let (total, torrents) = storage
        .torrents(
            Some((Sort::Modified, Order::Asc)),
            page.map(|p| if p > 0 { p - 1 } else { p } * storage.default_limit),
            Some(storage.default_limit),
        )
        .map_err(|e| Custom(Status::InternalServerError, e.to_string()))?;
    Ok(Template::render(
        "index",
        context! {
            meta: meta.inner(),
            back: page.map(|p| uri!(index(if p > 2 { Some(p - 1) } else { None }))),
            next: if page.unwrap_or(1) * storage.default_limit >= total { None }
                    else { Some(uri!(index(Some(page.map_or(2, |p| p + 1))))) },
            rows: torrents
                .into_iter()
                .map(|torrent| Row {
                    created: torrent
                        .creation_date
                        .map(|t| t.format(&meta.format_time).to_string()),
                    indexed: torrent.time.format(&meta.format_time).to_string(),
                    magnet: format::magnet(&torrent.info_hash, meta.trackers.as_ref()),
                    size: format::bytes(torrent.size),
                    files: torrent.files.as_ref().map_or("1 file".into(), |f| {
                        let l = f.len();
                        format!("{l} {}", l.plurify(&["file", "files", "files"]))
                    }),
                    torrent,
                })
                .collect::<Vec<Row>>(),
            pagination_totals: format!(
                "Page {} / {} ({total} {} total)",
                page.unwrap_or(1),
                (total as f64 / storage.default_limit as f64).ceil(),
                total.plurify(&["torrent", "torrents", "torrents"])
            )
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
        .1
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
        config.tracker.clone(),
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
            title: config.title,
            trackers: config.tracker,
        })
        .mount("/", routes![index, rss])
}
