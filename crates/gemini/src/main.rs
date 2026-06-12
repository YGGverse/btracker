mod config;
mod format;
mod route;

use anyhow::Result;
use btracker_fs::public::{Order, Sort, Storage, Torrent};
use btracker_scrape::Buffer as Scrape;
use config::Config;
use librqbit_core::torrent_metainfo::{TorrentMetaV1Owned, torrent_from_bytes};
use log::*;
use native_tls::{HandshakeError, Identity, TlsAcceptor, TlsStream};
use std::{
    collections::HashMap,
    fs::File,
    io::{Read, Write},
    net::{SocketAddr, TcpListener, TcpStream},
    path::PathBuf,
    sync::Arc,
    thread,
};
use tokio::sync::RwLock;

#[tokio::main]
async fn main() -> Result<()> {
    use chrono::Local;
    use clap::Parser;

    if std::env::var("RUST_LOG").is_ok() {
        use tracing_subscriber::{EnvFilter, fmt::*};
        struct T;
        impl time::FormatTime for T {
            fn format_time(&self, w: &mut format::Writer<'_>) -> std::fmt::Result {
                write!(w, "{}", Local::now())
            }
        }
        fmt()
            .with_timer(T)
            .with_env_filter(EnvFilter::from_default_env())
            .init()
    }

    let config = Config::parse();
    let state = Arc::new(State {
        public: Storage::init(&config.storage, config.limit, config.capacity).unwrap(),
        scrape: Scrape::new(
            config.scrape,
            config.scrape_timeout,
            config.scrape_proxy.as_ref(),
            config.scrape_proxy_i2p.as_ref(),
        )
        .unwrap(),
        format_date: config.format_date,
        name: config.name,
        description: config.description,
        tracker: config.tracker,
    });

    // https://geminiprotocol.net/docs/protocol-specification.gmi#the-use-of-tls
    let acceptor = TlsAcceptor::new(Identity::from_pkcs12(
        &{
            let mut buffer = vec![];
            File::open(&config.identity)?.read_to_end(&mut buffer)?;
            buffer
        },
        &config.password,
    )?)?;

    let listener = TcpListener::bind(config.bind)?;

    info!("Server started on `{}`", config.bind);

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                thread::spawn({
                    let state = state.clone();
                    let peer = stream.peer_addr()?;
                    let connection = acceptor.accept(stream);
                    move || handle(state, peer, connection)
                });
            }
            Err(e) => error!("{e}"),
        }
    }
    Ok(())
}

async fn handle(
    state: Arc<State>,
    peer: SocketAddr,
    connection: Result<TlsStream<TcpStream>, HandshakeError<TcpStream>>,
) {
    use titanite::*;
    debug!("Incoming connection from: `{peer}`");
    match connection {
        Ok(mut stream) => {
            // server should work with large files without memory overload,
            // because of that, incoming data read partially, using chunks;
            // collect header bytes first to route the request by its type.
            let mut header_buffer = Vec::with_capacity(HEADER_MAX_LEN);
            loop {
                let mut header_chunk = vec![0];
                match stream.read(&mut header_chunk) {
                    Ok(0) => warn!("Peer `{peer}` closed connection."),
                    Ok(l) => {
                        // validate header buffer, break on its length reached protocol limits
                        if header_buffer.len() + l > HEADER_MAX_LEN {
                            return send(
                                &response::failure::permanent::BadRequest {
                                    message: Some("Bad request".to_string()),
                                }
                                .into_bytes(),
                                &mut stream,
                                |result| match result {
                                    Ok(()) => warn!("Bad request from peer `{peer}`"),
                                    Err(e) => error!("Send packet to peer `{peer}` failed: {e}"),
                                },
                            );
                        }

                        // take chunk bytes at this point
                        header_buffer.extend(header_chunk);

                        // ending header byte received
                        if header_buffer.last().is_some_and(|&b| b == b'\n') {
                            // header bytes contain valid Gemini **request**
                            if let Ok(request) = request::Gemini::from_bytes(&header_buffer) {
                                return response(request, &state, &peer, &mut stream).await;
                            }

                            // header bytes received but yet could not be parsed,
                            // complete with request failure
                            send(
                                &response::failure::permanent::BadRequest {
                                    message: Some("Bad request".to_string()),
                                }
                                .into_bytes(),
                                &mut stream,
                                |result| match result {
                                    Ok(()) => warn!("Bad request from peer `{peer}`"),
                                    Err(e) => error!("Send packet to peer `{peer}` failed: {e}"),
                                },
                            )
                        }
                    }
                    Err(e) => send(
                        &response::failure::permanent::BadRequest {
                            message: Some("Bad request".to_string()),
                        }
                        .into_bytes(),
                        &mut stream,
                        |result| match result {
                            Ok(()) => warn!("Send failure response to peer `{peer}`: {e}"),
                            Err(e) => error!("Send packet to peer `{peer}` failed: {e}"),
                        },
                    ),
                }
            }
        }
        Err(e) => warn!("Handshake issue for peer `{peer}`: {e}"),
    }
}

async fn response(
    request: titanite::request::Gemini,
    state: &State,
    peer: &SocketAddr,
    stream: &mut TlsStream<TcpStream>,
) {
    use route::Route;
    use titanite::response::*;
    debug!("Incoming request from `{peer}` to `{}`", request.url.path());
    send(
        &match Route::from_url(&request.url, &state.public) {
            Route::File(ref path) => success::Default {
                data: &std::fs::read(path).unwrap(),
                meta: success::default::Meta {
                    mime: match path.extension() {
                        Some(extension) => {
                            let e = extension.to_ascii_lowercase();
                            if e == "jpeg" || e == "jpg" {
                                "image/jpeg"
                            } else if e == "gif" {
                                "image/gif"
                            } else if e == "png" {
                                "image/png"
                            } else if e == "webp" {
                                "image/webp"
                            } else if e == "txt" || e == "log" {
                                "text/plain"
                            } else if e == "gemini" || e == "gmi" {
                                "text/gemini"
                            } else {
                                todo!()
                            }
                        }
                        None => todo!(),
                    }
                    .to_string(),
                },
            }
            .into_bytes(),
            Route::List { page, keyword } => match list(state, keyword.as_deref(), page).await {
                Ok(data) => success::Default {
                    data: data.as_bytes(),
                    meta: success::default::Meta {
                        mime: "text/gemini".to_string(),
                    },
                }
                .into_bytes(),
                Err(e) => {
                    error!("Internal server error on handle peer `{peer}` request: `{e}`");
                    failure::temporary::General {
                        message: Some("Internal server error".to_string()),
                    }
                    .into_bytes()
                }
            },
            Route::Search => Input::Default(input::Default {
                message: Some("Keyword, file, hash...".into()),
            })
            .into_bytes(),
            Route::Info(id) => match state.public.torrent(id) {
                Some(torrent) => match info(state, torrent).await {
                    Ok(data) => success::Default {
                        data: data.as_bytes(),
                        meta: success::default::Meta {
                            mime: "text/gemini".to_string(),
                        },
                    }
                    .into_bytes(),
                    Err(e) => {
                        error!("Internal server error on handle peer `{peer}` request: `{e}`");
                        failure::temporary::General {
                            message: Some("Internal server error".to_string()),
                        }
                        .into_bytes()
                    }
                },
                None => {
                    warn!(
                        "Requested torrent `{}` not found by peer `{peer}`",
                        request.url.as_str()
                    );
                    Failure::Permanent(failure::Permanent::NotFound(failure::permanent::NotFound {
                        message: None,
                    }))
                    .into_bytes()
                }
            },
            Route::NotFound => {
                warn!(
                    "Requested resource `{}` not found by peer `{peer}`",
                    request.url.as_str()
                );
                Failure::Permanent(failure::Permanent::NotFound(failure::permanent::NotFound {
                    message: None,
                }))
                .into_bytes()
            }
        },
        stream,
        |result| {
            if let Err(e) = result {
                error!("Internal server error on handle peer `{peer}` request: `{e}`")
            }
        },
    )
}

fn send(data: &[u8], stream: &mut TlsStream<TcpStream>, callback: impl FnOnce(Result<()>)) {
    fn close(stream: &mut TlsStream<TcpStream>) -> Result<()> {
        stream.flush()?;
        // close connection gracefully
        // https://geminiprotocol.net/docs/protocol-specification.gmi#closing-connections
        stream.shutdown()?;
        Ok(())
    }
    callback((|| {
        stream.write_all(data)?;
        close(stream)?;
        Ok(())
    })());
}

async fn list(state: &State, keyword: Option<&str>, page: Option<usize>) -> Result<String> {
    /// format search keyword as the pagination query
    fn query(keyword: Option<&str>) -> String {
        keyword.map(|k| format!("?{}", k)).unwrap_or_default()
    }

    let scrape_index: Arc<RwLock<HashMap<[u8; 20], btracker_scrape::Result>>> =
        Arc::new(RwLock::new(HashMap::new())); // scrape info-hashes once

    let result = state
        .public
        .torrents(
            keyword,
            Some((Sort::Modified, Order::Desc)),
            page.map(|p| if p > 0 { p - 1 } else { p } * state.public.default_limit),
            Some(state.public.default_limit),
            {
                let si = scrape_index.clone();
                let keyword_exists = keyword.is_some();

                move |id20| {
                    let si = si.clone();
                    async move {
                        if let Ok(s) = state.scrape.get(&[id20.0]).await {
                            let is_active = s.incomplete > 0 || s.downloaded > 0 || s.complete > 0;
                            assert!(si.write().await.insert(id20.0, s).is_none());
                            keyword_exists || is_active
                        } else {
                            keyword_exists
                        }
                    }
                }
            },
        )
        .await?;

    let mut b = Vec::new();

    b.push(format!("# {}\n", {
        let mut h = String::new();

        if let Some(k) = keyword {
            h.push_str(k);
            h.push_str(" • ");
        }

        if let Some(p) = page
            && p > 1
        {
            h.push_str(&format!("Page {p} • "));
        }

        h.push_str(&state.name);
        h
    }));

    if let Some(ref description) = state.description {
        b.push(format!("{description}\n"));
    }

    if let Some(ref trackers) = state.tracker {
        b.push("```".into());
        for tracker in trackers {
            b.push(tracker.to_string());
        }
        b.push("```\n".into());
    }

    b.push("## Recent\n".into());

    if result.list.is_empty() {
        b.push("Nothing.\n".into())
    } else {
        let mut si = scrape_index.write().await;
        for torrent in result.list {
            let i: TorrentMetaV1Owned = torrent_from_bytes(&torrent.bytes)?;
            b.push(format!(
                "=> /{} {}",
                i.info_hash.as_string(),
                i.info
                    .name
                    .as_ref()
                    .map(|n| n.to_string())
                    .unwrap_or_default()
            ));
            b.push(format!(
                "{} • {} • {}",
                torrent.time.format(&state.format_date),
                format::total(&i),
                format::files(&i)
            ));
            if let Some(s) = si.remove(&i.info_hash.0) {
                b.push(format!(
                    " • ↑ {} ↓ {} ⏲ {}",
                    s.complete, s.downloaded, s.incomplete
                ))
            }
        }
    }

    b.push("## Navigation\n".into());

    if keyword.is_none() {
        b.push(format!(
            "Page {} / {} ({} active {} total)\n",
            page.unwrap_or(1),
            (result.visible as f64 / state.public.default_limit as f64).ceil(),
            result.visible,
            result.total
        ));
    } else {
        b.push(format!(
            "Page {} / {} ({} total)\n",
            page.unwrap_or(1),
            (result.visible as f64 / state.public.default_limit as f64).ceil(),
            result.visible
        ));
    }

    if page.unwrap_or(1) * state.public.default_limit < result.visible {
        b.push(format!(
            "=> /{}{} Next",
            page.map_or(2, |p| p + 1),
            query(keyword)
        ))
    }

    if let Some(p) = page {
        b.push(format!(
            "=> {}{} Back",
            if p > 2 {
                format!("/{}", p - 1)
            } else {
                "/".into()
            },
            query(keyword)
        ))
    }

    b.push("\n=> /search Search".into());

    Ok(b.join("\n"))
}

async fn info(state: &State, torrent: Torrent) -> Result<String> {
    struct File {
        path: Option<PathBuf>,
        length: u64,
    }
    impl File {
        pub fn path(&self) -> String {
            self.path
                .as_ref()
                .map(|p| p.to_string_lossy().into())
                .unwrap_or("?".into())
        }
    }

    let i: TorrentMetaV1Owned = torrent_from_bytes(&torrent.bytes)?;

    let mut b = Vec::new();

    b.push(format!(
        "# {} • {}\n",
        i.info
            .name
            .as_ref()
            .map(|n| n.to_string())
            .unwrap_or(state.name.clone()),
        state.name
    ));

    let t = state.scrape.get(&[i.info_hash.0]).await.unwrap_or_default();
    b.push(format!(
        "{} • {} • {}{}\n",
        torrent.time.format(&state.format_date),
        format::total(&i),
        format::files(&i),
        format!(" • ↑ {} ↓ {} ⏲ {}", t.complete, t.downloaded, t.incomplete)
    ));

    b.push(format!(
        "=> {} Magnet\n",
        format::magnet(&i, state.tracker.as_ref())
    ));

    if let Some(files) = i.info.files.map(|files| {
        let mut b = Vec::with_capacity(files.len());
        for f in files {
            let mut p = PathBuf::new();
            b.push(File {
                length: f.length,
                path: match f.full_path(&mut p) {
                    Ok(()) => Some(p),
                    Err(e) => {
                        warn!("Filename decode error: {e}");
                        None
                    }
                },
            })
        }
        b.sort_by(|a, b| a.path.cmp(&b.path)); // @TODO optional
        b
    }) {
        b.push("## Files\n".into());
        for file in files {
            let p = file.path();
            b.push(match state.public.href(&i.info_hash.as_string(), &p) {
                Some(href) => format!(
                    "=> {} {} ({})",
                    urlencoding::encode(&href),
                    p,
                    format::size(file.length)
                ),
                None => format!("{} ({})", p, format::size(file.length)), // * ?
            })
        }
    }

    Ok(b.join("\n"))
}

struct State {
    description: Option<String>,
    format_date: String,
    name: String,
    public: Storage,
    scrape: Scrape,
    tracker: Option<Vec<url::Url>>,
}
