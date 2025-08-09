#[macro_use]
extern crate rocket;

mod config;
mod feed;
mod meta;
mod scraper;
mod storage;
mod torrent;

use config::Config;
use feed::Feed;
use meta::Meta;
use plurify::Plurify;
use rocket::{
    State,
    http::Status,
    response::{content::RawXml, status::Custom},
    serde::Serialize,
};
use rocket_dyn_templates::{Template, context};
use scraper::{Scrape, Scraper};
use std::str::FromStr;
use storage::{Order, Sort, Storage};
use torrent::Torrent;

#[get("/?<page>")]
fn index(
    page: Option<usize>,
    scraper: &State<Scraper>,
    storage: &State<Storage>,
    meta: &State<Meta>,
) -> Result<Template, Custom<String>> {
    #[derive(Serialize)]
    #[serde(crate = "rocket::serde")]
    struct R {
        created: Option<String>,
        files: String,
        indexed: String,
        magnet: String,
        scrape: Option<Scrape>,
        size: String,
        torrent: Torrent,
    }
    let (total, torrents) = storage
        .torrents(
            Some((Sort::Modified, Order::Desc)),
            page.map(|p| if p > 0 { p - 1 } else { p } * storage.default_limit),
            Some(storage.default_limit),
        )
        .map_err(|e| {
            error!("Torrents storage read error: `{e}`");
            Custom(Status::InternalServerError, E.to_string())
        })?;
    Ok(Template::render(
        "index",
        context! {
            meta: meta.inner(),
            back: page.map(|p| uri!(index(if p > 2 { Some(p - 1) } else { None }))),
            next: if page.unwrap_or(1) * storage.default_limit >= total { None }
                    else { Some(uri!(index(Some(page.map_or(2, |p| p + 1))))) },
            rows: torrents
                .into_iter()
                .filter_map(|t| match Torrent::from_storage(&t.bytes, t.time) {
                    Ok(torrent) => Some(R {
                        created: torrent.creation_date.map(|t| t.format(&meta.format_time).to_string()),
                        files: torrent.files(),
                        indexed: torrent.time.format(&meta.format_time).to_string(),
                        magnet: torrent.magnet(meta.trackers.as_ref()),
                        scrape: scraper.scrape(&torrent.info_hash),
                        size: torrent.size(),
                        torrent
                    }),
                    Err(e) => {
                        error!("Torrent storage read error: `{e}`");
                        None
                    }
                })
                .collect::<Vec<R>>(),
            pagination_totals: format!(
                "Page {} / {} ({total} {} total)",
                page.unwrap_or(1),
                (total as f64 / storage.default_limit as f64).ceil(),
                total.plurify(&["torrent", "torrents", "torrents"])
            )
        },
    ))
}

#[get("/<info_hash>")]
fn info(
    info_hash: &str,
    storage: &State<Storage>,
    scraper: &State<Scraper>,
    meta: &State<Meta>,
) -> Result<Template, Custom<String>> {
    match storage.torrent(librqbit_core::Id20::from_str(info_hash).map_err(|e| {
        warn!("Torrent info-hash parse error: `{e}`");
        Custom(Status::BadRequest, Status::BadRequest.to_string())
    })?) {
        Some(t) => {
            #[derive(Serialize)]
            #[serde(crate = "rocket::serde")]
            struct F {
                name: String,
                size: String,
            }
            let torrent = Torrent::from_storage(&t.bytes, t.time).map_err(|e| {
                error!("Torrent parse error: `{e}`");
                Custom(Status::InternalServerError, E.to_string())
            })?;
            Ok(Template::render(
                "info",
                context! {
                    meta: meta.inner(),
                    created: torrent.creation_date.map(|t| t.format(&meta.format_time).to_string()),
                    files_total: torrent.files(),
                    files_list: torrent.files.as_ref().map(|f| {
                        f.iter()
                            .map(|f| F {
                                name: f.name(),
                                size: f.size(),
                            })
                            .collect::<Vec<F>>()
                    }),
                    indexed: torrent.time.format(&meta.format_time).to_string(),
                    magnet: torrent.magnet(meta.trackers.as_ref()),
                    scrape: scraper.scrape(info_hash),
                    size: torrent.size(),
                    torrent
                },
            ))
        }
        None => Err(Custom(Status::NotFound, E.to_string())),
    }
}

#[get("/rss")]
fn rss(feed: &State<Feed>, storage: &State<Storage>) -> Result<RawXml<String>, Custom<String>> {
    let mut b = feed.transaction(1024); // @TODO
    for t in storage
        .torrents(
            Some((Sort::Modified, Order::Desc)),
            None,
            Some(storage.default_limit),
        )
        .map_err(|e| {
            error!("Torrent storage read error: `{e}`");
            Custom(Status::InternalServerError, E.to_string())
        })?
        .1
    {
        feed.push(
            &mut b,
            Torrent::from_storage(&t.bytes, t.time).map_err(|e| {
                error!("Torrent parse error: `{e}`");
                Custom(Status::InternalServerError, E.to_string())
            })?,
        )
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
        config.canonical_url.clone(),
        config.tracker.clone(),
    );
    let scraper = Scraper::init(
        config
            .scrape
            .map(|u| {
                u.into_iter()
                    .map(|url| {
                        use std::str::FromStr;
                        if url.scheme() == "tcp" {
                            todo!("TCP scrape is not implemented")
                        }
                        if url.scheme() != "udp" {
                            todo!("Scheme `{}` is not supported", url.scheme())
                        }
                        std::net::SocketAddr::new(
                            std::net::IpAddr::from_str(
                                url.host_str()
                                    .expect("Required valid host value")
                                    .trim_start_matches('[')
                                    .trim_end_matches(']'),
                            )
                            .unwrap(),
                            url.port().expect("Required valid port value"),
                        )
                    })
                    .collect()
            })
            .map(|a| (config.udp, a)),
    );
    let storage = Storage::init(config.preload, config.list_limit, config.capacity).unwrap();
    rocket::build()
        .attach(Template::fairing())
        .configure(rocket::Config {
            port: config.port,
            address: config.host,
            ..if config.debug {
                rocket::Config::debug_default()
            } else {
                rocket::Config::default()
            }
        })
        .manage(feed)
        .manage(scraper)
        .manage(storage)
        .manage(Meta {
            canonical: config.canonical_url,
            description: config.description,
            format_time: config.format_time,
            title: config.title,
            trackers: config.tracker,
            version: env!("CARGO_PKG_VERSION").into(),
        })
        .mount("/", rocket::fs::FileServer::from(config.statics))
        .mount("/", routes![index, info, rss])
}

/// Public placeholder text for the `Status::InternalServerError`
const E: &str = "Oops!";
