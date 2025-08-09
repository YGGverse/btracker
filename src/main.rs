#[macro_use]
extern crate rocket;

mod config;
mod feed;
mod meta;
mod public;
mod scraper;
mod torrent;

use config::Config;
use feed::Feed;
use meta::Meta;
use plurify::Plurify;
use public::{Order, Public, Sort};
use rocket::{
    State,
    http::Status,
    response::{content::RawXml, status::Custom},
    serde::Serialize,
};
use rocket_dyn_templates::{Template, context};
use scraper::{Scrape, Scraper};
use std::str::FromStr;
use torrent::Torrent;

#[get("/?<page>")]
fn index(
    page: Option<usize>,
    scraper: &State<Scraper>,
    public: &State<Public>,
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
    let (total, torrents) = public
        .torrents(
            Some((Sort::Modified, Order::Desc)),
            page.map(|p| if p > 0 { p - 1 } else { p } * public.default_limit),
            Some(public.default_limit),
        )
        .map_err(|e| {
            error!("Torrents public storage read error: `{e}`");
            Custom(Status::InternalServerError, E.to_string())
        })?;
    Ok(Template::render(
        "index",
        context! {
            meta: meta.inner(),
            back: page.map(|p| uri!(index(if p > 2 { Some(p - 1) } else { None }))),
            next: if page.unwrap_or(1) * public.default_limit >= total { None }
                    else { Some(uri!(index(Some(page.map_or(2, |p| p + 1))))) },
            rows: torrents
                .into_iter()
                .filter_map(|t| match Torrent::from_public(&t.bytes, t.time) {
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
                (total as f64 / public.default_limit as f64).ceil(),
                total.plurify(&["torrent", "torrents", "torrents"])
            )
        },
    ))
}

#[get("/<info_hash>")]
fn info(
    info_hash: &str,
    public: &State<Public>,
    scraper: &State<Scraper>,
    meta: &State<Meta>,
) -> Result<Template, Custom<String>> {
    match public.torrent(librqbit_core::Id20::from_str(info_hash).map_err(|e| {
        warn!("Torrent info-hash parse error: `{e}`");
        Custom(Status::BadRequest, Status::BadRequest.to_string())
    })?) {
        Some(t) => {
            #[derive(Serialize)]
            #[serde(crate = "rocket::serde")]
            struct F {
                href: Option<String>,
                path: String,
                size: String,
            }
            let torrent = Torrent::from_public(&t.bytes, t.time).map_err(|e| {
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
                            .map(|f| {
                                let p = f.path();
                                F {
                                    href: public.href(&torrent.info_hash, &p),
                                    path: p,
                                    size: f.size(),
                                }
                            })
                            .collect::<Vec<F>>()
                    }),
                    indexed: torrent.time.format(&meta.format_time).to_string(),
                    magnet: torrent.magnet(meta.trackers.as_ref()),
                    scrape: scraper.scrape(&torrent.info_hash),
                    size: torrent.size(),
                    torrent
                },
            ))
        }
        None => Err(Custom(Status::NotFound, E.to_string())),
    }
}

#[get("/rss")]
fn rss(meta: &State<Meta>, public: &State<Public>) -> Result<RawXml<String>, Custom<String>> {
    let mut f = Feed::new(
        &meta.title,
        meta.description.as_deref(),
        meta.canonical.clone(),
        1024, // @TODO
    );
    for t in public
        .torrents(
            Some((Sort::Modified, Order::Desc)),
            None,
            Some(public.default_limit),
        )
        .map_err(|e| {
            error!("Torrent public storage read error: `{e}`");
            Custom(Status::InternalServerError, E.to_string())
        })?
        .1
    {
        f.push(Torrent::from_public(&t.bytes, t.time).map_err(|e| {
            error!("Torrent parse error: `{e}`");
            Custom(Status::InternalServerError, E.to_string())
        })?)
    }
    Ok(RawXml(f.commit()))
}

#[launch]
fn rocket() -> _ {
    use clap::Parser;
    let config = Config::parse();
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
        .manage(scraper)
        .manage(Public::init(config.public.clone(), config.list_limit, config.capacity).unwrap())
        .manage(Meta {
            canonical: config.canonical_url,
            description: config.description,
            format_time: config.format_time,
            title: config.title,
            trackers: config.tracker,
            version: env!("CARGO_PKG_VERSION").into(),
        })
        .mount("/", rocket::fs::FileServer::from(config.public))
        .mount("/", routes![index, info, rss])
}

/// Public placeholder text for the `Status::InternalServerError`
const E: &str = "Oops!";
