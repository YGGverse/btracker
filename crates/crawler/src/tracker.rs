use anyhow::Result;
use btpeer::Peer;
use chrono::Utc;
use librqbit::dht::Id20;
use log::*;
use std::{
    collections::{HashMap, HashSet},
    net::{IpAddr, SocketAddr},
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::Duration,
};
use tokio::sync::RwLock;
use url::Url;
use yosemite::{Session, style::Stream};

pub enum Tracker {
    Default {
        peers_limit: Option<usize>,
        port: u16,
        proxy: Option<Url>,
        timeout: Duration,
        url: Url,
    },
    I2p {
        announce_timeout: Duration,
        loopback: IpAddr,
        peer_connect_timeout: Duration,
        peers_limit: Option<usize>,
        peers_map: Arc<RwLock<HashMap<String, I2pSession>>>,
        port: u16,
        proxy: Option<Url>,
        sam_session: Arc<RwLock<Session<Stream>>>,
        url: Url,
    },
}

impl Tracker {
    async fn peers(&self, info_hash: &Id20) -> Result<HashSet<SocketAddr>> {
        Ok(match self {
            Self::Default {
                peers_limit,
                port,
                proxy,
                timeout,
                url,
            } => {
                let announce =
                    btpeer::http::query::Announce::new(url.as_str(), &info_hash.0, *port)?;

                let peers = take_random_peers(
                    btpeer::http::announce(&announce, *timeout, proxy.as_ref().map(|u| u.as_str()))
                        .await?
                        .peers
                        .0
                        .into_iter()
                        .filter(|p| match p {
                            Peer::Default(this) => {
                                url.host_str()
                                    .is_some_and(|h| !h.contains(&this.host.to_string()))
                                    && this.port != *port // exclude self
                            }
                            Peer::I2p(..) => false,
                        })
                        .collect(),
                    *peers_limit,
                );

                let mut b = HashSet::with_capacity(peers.len());

                for p in peers {
                    match p {
                        Peer::Default(peer) => {
                            let p = SocketAddr::new(peer.host, peer.port);
                            if b.insert(p) {
                                debug!("[tracker] add peer: {p}")
                            } else {
                                debug!("[tracker] replace existing peer: {p}")
                            }
                        }
                        Peer::I2p(peer) => {
                            unreachable!(
                                "[tracker] unexpected peer {peer} from tracker {url}, skip"
                            )
                        }
                    }
                }
                b
            }
            Self::I2p {
                announce_timeout,
                loopback,
                peer_connect_timeout,
                peers_limit,
                peers_map,
                port,
                proxy,
                sam_session,
                url,
            } => {
                let announce =
                    btpeer::http::query::Announce::new(url.as_str(), &info_hash.0, *port)?;

                let b32 = b32(sam_session.read().await.destination().as_bytes());

                let peers = take_random_peers(
                    btpeer::http::announce_i2p(
                        &announce,
                        *announce_timeout,
                        proxy.as_ref().map(|u| u.as_str()),
                    )
                    .await?
                    .peers
                    .0
                    .into_iter()
                    .filter(|p| match p {
                        Peer::I2p(this) => this.b32 != b32, // exclude self
                        Peer::Default(..) => false,
                    })
                    .collect(),
                    *peers_limit,
                );

                let mut m = peers_map.write().await; // prevents infinitive async socket spawn
                let mut b = HashSet::with_capacity(peers.len()); // resulting peers buffer

                for p in peers {
                    match p {
                        Peer::I2p(peer) => {
                            if let Some(i2p_session) = m.get(&peer.b32) {
                                i2p_session
                                    .last_active
                                    .store(Utc::now().timestamp() as u64, Ordering::Relaxed);

                                b.insert(i2p_session.socket);
                                debug!(
                                    "[tracker] reuse existing I2P peer {peer} as {}",
                                    i2p_session.socket
                                );
                                continue;
                            }

                            debug!("[tracker] init SAM proxy for {peer} on {loopback}...");

                            let listener =
                                tokio::net::TcpListener::bind(SocketAddr::new(*loopback, 0))
                                    .await?;

                            let socket = listener.local_addr()?;

                            if b.insert(socket) {
                                debug!("[tracker] bind I2P peer {peer} as {socket}")
                            } else {
                                debug!("[tracker] bind existing I2P peer {peer} as {socket}")
                            }

                            debug!(
                                "[tracker] listening incoming connections for {peer} on {socket} as {b32}...",
                            );

                            let timeout = *peer_connect_timeout;
                            let session = sam_session.clone();
                            let peer_b32 = peer.b32.clone();
                            let handler = tokio::spawn(async move {
                                while let Ok((mut local, client)) = listener.accept().await {
                                    debug!(
                                        "[tracker] accepting SAM connection from {client} ({peer_b32})"
                                    );
                                    match tokio::time::timeout(
                                        timeout,
                                        session.write().await.connect(&peer_b32),
                                    )
                                    .await
                                    {
                                        Ok(connection) => match connection {
                                            Ok(mut remote) => {
                                                debug!(
                                                    "[tracker] begin SAM connection to {}",
                                                    remote.remote_destination() // | &peer_b32
                                                );
                                                match tokio::io::copy_bidirectional(
                                                    &mut local,
                                                    &mut remote,
                                                )
                                                .await // @TODO timeout?
                                                {
                                                    Ok((a, b)) => trace!(
                                                        "[tracker] copied {a}/{b} to {}",
                                                        remote.remote_destination() // | &peer_b32
                                                    ),
                                                    Err(e) => debug!("{e}"),
                                                }
                                            }
                                            Err(e) => debug!(
                                                "[tracker] connection failed to {client} ({peer_b32}): {e}"
                                            ),
                                        },
                                        Err(e) => debug!(
                                            "[tracker] connection to {client} ({peer_b32}) timed out after {} seconds: {e}",
                                            timeout.as_secs()
                                        ),
                                    }
                                }
                            });
                            assert!(
                                m.insert(
                                    peer.b32,
                                    I2pSession {
                                        socket,
                                        handler,
                                        last_active: AtomicU64::new(Utc::now().timestamp() as u64),
                                    },
                                )
                                .is_none()
                            )
                        }
                        Peer::Default(peer) => {
                            warn!("[tracker] unexpected peer {peer} from I2P tracker {url}, skip")
                        }
                    }
                }
                b
            }
        })
    }

    fn url(&self) -> &Url {
        match self {
            Self::Default { url, .. } => url,
            Self::I2p { url, .. } => url,
        }
    }
}

pub struct I2pSession {
    pub handler: tokio::task::JoinHandle<()>,
    pub last_active: AtomicU64,
    pub socket: SocketAddr,
}

pub struct Buffer(pub Vec<Tracker>);

impl Buffer {
    /// Return peers from trackers
    pub async fn peers(&self, info_hash: &Id20) -> Result<HashSet<SocketAddr>> {
        let mut peers = HashSet::new(); // unique peers buffer collected from all trackers

        for tracker in self.0.iter() {
            debug!(
                "[tracker] get peers from {} for {}...",
                tracker.url(),
                info_hash.as_string(),
            );
            peers.extend(tracker.peers(info_hash).await?)
        }

        Ok(peers)
    }

    /// Build magnet URI (`librqbit` impl dependency)
    pub fn magnet(&self, info_hash: &str) -> String {
        format!("magnet:?xt=urn:btih:{info_hash}")
    }
}

fn take_random_peers(mut peers: Vec<Peer>, limit: Option<usize>) -> Vec<Peer> {
    use rand::seq::SliceRandom;

    let total = peers.len();

    let mut rng = rand::rng();
    peers.shuffle(&mut rng);

    match limit {
        Some(l) => {
            let p: Vec<Peer> = peers.into_iter().take(l).collect();
            debug!(
                "[tracker] taken random peers: {}/{total} (limited to {l} max)",
                p.len()
            );
            p
        }
        None => {
            debug!("[tracker] taken random peers: {total}");
            peers
        }
    }
}

fn b32(destination: &[u8]) -> String {
    use data_encoding::BASE32_NOPAD;
    use sha2::{Digest, Sha256};

    let mut hasher = Sha256::new();
    hasher.update(destination);
    let hash_result = hasher.finalize();

    format!(
        "{}.b32.i2p",
        BASE32_NOPAD.encode(&hash_result).to_lowercase()
    )
}
