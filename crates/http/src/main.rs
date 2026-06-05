#[macro_use]
extern crate rocket;

mod config;
mod feed;
mod info_hash;
mod meta;
mod scrape;
mod torrent;

use btracker_fs::public::{Order, Sort, Storage};
use btracker_scrape::Scrape;
use config::Config;
use feed::Feed;
use info_hash::InfoHash;
use meta::Meta;
use rocket::{
    State,
    http::{ContentType, Header, Status},
    response::{Responder, Response, content::RawXml},
    serde::Serialize,
};
use rocket_dyn_templates::{Template, context};
use torrent::Torrent;

#[get("/?<search>&<page>")]
fn index(
    search: Option<&str>,
    page: Option<usize>,
    scrape: &State<Scrape>,
    storage: &State<Storage>,
    meta: &State<Meta>,
) -> Result<Template, Status> {
    use std::{cell::RefCell, collections::HashMap, rc::Rc};

    #[derive(Serialize)]
    #[serde(crate = "rocket::serde")]
    struct R {
        created: Option<String>,
        files: Option<usize>,
        indexed: String,
        magnet: String,
        torrent: String,
        scrape: Option<scrape::Result>,
        size: String,
        this: Torrent,
    }

    let scrape_index: Rc<RefCell<HashMap<librqbit_core::Id20, scrape::Result>>> =
        Rc::new(RefCell::new(HashMap::new())); // scrape info-hashes once

    let result = storage
        .torrents(
            search,
            Some((Sort::Modified, Order::Desc)),
            page.map(|p| if p > 0 { p - 1 } else { p } * storage.default_limit),
            Some(storage.default_limit),
            {
                let si = scrape_index.clone();
                move |id| {
                    scrape::get(scrape, id.0).is_none_or(|s| {
                        let is_active = s.leechers > 0 || s.peers > 0 || s.seeders > 0;
                        assert!(si.borrow_mut().insert(id, s).is_none());
                        search.is_some() || is_active
                    })
                }
            },
        )
        .map_err(|e| {
            error!("Torrents public storage read error: `{e}`");
            Status::InternalServerError
        })?;

    Ok(Template::render(
        "index",
        context! {
            title: {
                let mut t = String::new();
                if let Some(q) = search && !q.is_empty() {
                    t.push_str(q);
                    t.push_str(S);
                    t.push_str("Search");
                    t.push_str(S)
                }
                if let Some(p) = page && p > 1 {
                    t.push_str(&format!("Page {p}"));
                    t.push_str(S)
                }
                t.push_str(&meta.title);
                if let Some(ref description) = meta.description
                        && page.is_none_or(|p| p == 1) && search.is_none_or(|q| q.is_empty()) {
                    t.push_str(S);
                    t.push_str(description)
                }
                t
            },
            meta: meta.inner(),
            back: page.map(|p| uri!(index(search, if p > 2 { Some(p - 1) } else { None }))),
            next: if page.unwrap_or(1) * storage.default_limit >= result.visible { None }
                    else { Some(uri!(index(search, Some(page.map_or(2, |p| p + 1))))) },
            rows: result.list
                .into_iter()
                .filter_map(|t| match Torrent::from_public(&t.bytes, t.time) {
                    Ok(this) => Some(R {
                        created: this.creation_date.map(|t| t.format(&meta.format_time).to_string()),
                        files: this.files(),
                        indexed: this.time.format(&meta.format_time).to_string(),
                        magnet: this.magnet(meta.trackers.as_ref()),
                        torrent: this.torrent(), // @TODO customize trackers
                        scrape: scrape_index.borrow_mut().remove(&this.id),
                        size: this.size(),
                        this
                    }),
                    Err(e) => {
                        error!("Torrent storage read error: `{e}`");
                        None
                    }
                })
                .collect::<Vec<R>>(),
            page: page.unwrap_or(1),
            pages: (result.visible as f64 / storage.default_limit as f64).ceil(),
            total: result.total,
            visible: result.visible,
            is_search: search.is_some(),
            search
        },
    ))
}

#[get("/<info_hash>", rank = 1)]
fn info(
    info_hash: InfoHash,
    storage: &State<Storage>,
    scrape: &State<Scrape>,
    meta: &State<Meta>,
) -> Result<Template, Status> {
    match storage.torrent(info_hash.id20()) {
        Some(t) => {
            #[derive(Serialize)]
            #[serde(crate = "rocket::serde")]
            struct F {
                href: Option<String>,
                path: String,
                size: String,
            }
            let this = Torrent::from_public(&t.bytes, t.time).map_err(|e| {
                error!("Torrent parse error: `{e}`");
                Status::InternalServerError
            })?;
            Ok(Template::render(
                "info",
                context! {
                    title: {
                        let mut t = String::new();
                        if let Some(ref name) = this.name {
                            t.push_str(name);
                            t.push_str(S)
                        }
                        t.push_str(&meta.title);
                        t
                    },
                    meta: meta.inner(),
                    created: this.creation_date.map(|t| t.format(&meta.format_time).to_string()),
                    files_total: this.files(),
                    files_list: this.files.as_ref().map(|f| {
                        f.iter()
                            .map(|f| {
                                let p = f.path();
                                F {
                                    href: storage.href(&this.info_hash, &p),
                                    path: p,
                                    size: f.size(),
                                }
                            })
                            .collect::<Vec<F>>()
                    }),
                    indexed: this.time.format(&meta.format_time).to_string(),
                    magnet: this.magnet(meta.trackers.as_ref()),
                    torrent: this.torrent(), // @TODO customize trackers
                    scrape: scrape::get(scrape, info_hash.bytes20()),
                    size: this.size(),
                    this
                },
            ))
        }
        None => Err(Status::NotFound),
    }
}

/// Return .torrent file with updated trackers @TODO resolve rank collision
#[get("/<filename>", rank = 2)]
fn torrent_file(
    filename: info_hash::Torrent,
    meta: &State<Meta>,
    storage: &State<Storage>,
) -> Result<TorrentFile, Status> {
    match storage.torrent(filename.id20()) {
        Some(t) => {
            use serde_bencode::value::Value;
            let mut b: std::collections::BTreeMap<String, serde_bencode::value::Value> =
                serde_bencode::from_bytes(&t.bytes).map_err(|e| {
                    error!("Torrent bytes decode error: `{e}`");
                    Status::InternalServerError
                })?;
            if let Some(ref trackers) = meta.trackers {
                let mut i = trackers.iter();
                if let Some(a) = i.next() {
                    b.insert(
                        "announce".to_string(),
                        Value::Bytes(a.as_str().as_bytes().to_vec()),
                    );
                }
                let mut l = Vec::new();
                for tracker in i {
                    l.push(Value::List(vec![Value::Bytes(
                        tracker.as_str().as_bytes().to_vec(),
                    )]))
                }
                b.insert("announce-list".to_string(), Value::List(l));
            }
            b.insert(
                "comment".to_string(),
                Value::Bytes(
                    format!(
                        "{}{}{}",
                        meta.title,
                        meta.description
                            .as_ref()
                            .map(|d| format!("\n{d}"))
                            .unwrap_or_default(),
                        meta.canonical
                            .as_ref()
                            .map(|c| format!("\n{c}"))
                            .unwrap_or_default(),
                    )
                    .as_bytes()
                    .to_vec(),
                ),
            );
            Ok(TorrentFile {
                name: format!(
                    "{}.torrent",
                    if let Some(Value::Dict(info)) = b.get("info") {
                        if let Some(Value::Bytes(name_bytes)) = info.get(b"name".as_slice()) {
                            String::from_utf8(name_bytes.clone())
                                .unwrap_or(filename.id20().as_string())
                        } else {
                            filename.id20().as_string()
                        }
                    } else {
                        filename.id20().as_string()
                    }
                ),
                data: serde_bencode::to_bytes(&b).map_err(|e| {
                    error!("Could not encode torrent bytes: `{e}`");
                    Status::InternalServerError
                })?,
            })
        }
        None => Err(Status::NotFound),
    }
}

#[get("/rss")]
fn rss(
    meta: &State<Meta>,
    scrape: &State<Scrape>,
    storage: &State<Storage>,
) -> Result<RawXml<String>, Status> {
    let mut f = Feed::new(
        &meta.title,
        meta.description.as_deref(),
        meta.canonical.clone(),
        1024, // @TODO
    );
    for t in storage
        .torrents(
            None,
            Some((Sort::Modified, Order::Desc)),
            None,
            Some(storage.default_limit),
            |id| {
                scrape::get(scrape, id.0)
                    .is_some_and(|s| s.leechers > 0 || s.peers > 0 || s.seeders > 0)
            },
        )
        .map_err(|e| {
            error!("Torrent storage read error: `{e}`");
            Status::InternalServerError
        })?
        .list
    {
        f.push(Torrent::from_public(&t.bytes, t.time).map_err(|e| {
            error!("Torrent parse error: `{e}`");
            Status::InternalServerError
        })?)
    }
    Ok(RawXml(f.commit()))
}

#[launch]
fn rocket() -> _ {
    use clap::Parser;
    let config = Config::parse();
    if config.canonical_url.is_none() {
        warn!("Canonical URL option is required for the RSS feed by the specification!") // @TODO
    }
    let scrape = Scrape::init(
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
                rocket::Config::release_default()
            }
        })
        .manage(scrape)
        .manage(Storage::init(&config.public, config.list_limit, config.capacity).unwrap())
        .manage(Meta {
            canonical: config.canonical_url,
            description: config.description,
            format_time: config.format_time,
            title: config.title,
            trackers: config.tracker,
            version: env!("CARGO_PKG_VERSION").into(),
        })
        .mount("/", rocket::fs::FileServer::from(config.public))
        .mount("/", routes![index, rss, info, torrent_file])
}

const S: &str = " • ";

/// Downloadable .torrent bytes, with meta-info updated
struct TorrentFile {
    name: String,
    data: Vec<u8>,
}
impl<'r> Responder<'r, 'static> for TorrentFile {
    fn respond_to(self, _: &'r rocket::request::Request<'_>) -> rocket::response::Result<'static> {
        Response::build()
            .header(ContentType::new("application", "x-bittorrent"))
            .header(Header::new(
                "Content-Disposition",
                format!("attachment; filename=\"{}\"", self.name),
            ))
            .sized_body(self.data.len(), std::io::Cursor::new(self.data))
            .ok()
    }
}
