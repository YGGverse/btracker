use anyhow::Result;
use btpeer::Peer;
use librqbit::dht::Id20;
use log::*;
use std::{
    collections::HashSet,
    net::{IpAddr, SocketAddr},
    sync::Arc,
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
        loopback: IpAddr,
        peers_limit: Option<usize>,
        port: u16,
        proxy: Option<Url>,
        timeout: Duration,
        url: Url,
        sam: Arc<RwLock<Session<Stream>>>,
    },
}

impl Tracker {
    async fn peers(
        &self,
        info_hash: &Id20,
        peers_b32: &mut HashSet<String>,
    ) -> Result<HashSet<SocketAddr>> {
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
                                debug!("[tracker] add peer: `{p}`")
                            } else {
                                debug!("[tracker] replace existing peer: `{p}`")
                            }
                        }
                        Peer::I2p(peer) => {
                            unreachable!(
                                "[tracker] unexpected peer `{peer}` from tracker `{url}`, skip"
                            )
                        }
                    }
                }
                b
            }
            Self::I2p {
                loopback,
                peers_limit,
                port,
                proxy,
                sam,
                timeout,
                url,
            } => {
                let announce =
                    btpeer::http::query::Announce::new(url.as_str(), &info_hash.0, *port)?;

                let b32 = b32(sam.read().await.destination().as_bytes());

                let peers = take_random_peers(
                    btpeer::http::announce_i2p(
                        &announce,
                        *timeout,
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

                let mut b = HashSet::with_capacity(peers.len());

                for p in peers {
                    match p {
                        Peer::I2p(peer) => {
                            if !peers_b32.insert(peer.b32.clone()) {
                                debug!(
                                    "[tracker] b32 value `{}` for peer `{peer}` on `{loopback}` exists, skip.",
                                    &peer.b32
                                );
                                continue;
                            }

                            debug!("[tracker] init SAM proxy for `{peer}` on `{loopback}`...");

                            let listener =
                                tokio::net::TcpListener::bind(SocketAddr::new(*loopback, 0))
                                    .await?;

                            let p = listener.local_addr()?;

                            if b.insert(p) {
                                debug!("[tracker] bind I2P peer `{peer}` as `{p}`")
                            } else {
                                debug!("[tracker] bind existing I2P peer `{peer}` as `{p}`")
                            }

                            debug!(
                                "[tracker] listening incoming connections for `{peer}` on `{p}` as `{b32}`...",
                            );

                            let session = sam.clone();
                            tokio::spawn(async move {
                                while let Ok((mut local, client)) = listener.accept().await {
                                    debug!(
                                        "[tracker] accepting SAM connection from {client} ({})",
                                        &peer.b32
                                    );
                                    if let Ok(mut remote) =
                                        session.write().await.connect(&peer.b32).await
                                    {
                                        debug!(
                                            "[tracker] begin SAM connection to `{}`",
                                            remote.remote_destination() // | &peer.b32
                                        );
                                        match tokio::io::copy_bidirectional(&mut local, &mut remote)
                                            .await
                                        {
                                            Ok((a, b)) => trace!(
                                                "[tracker] copied {a}/{b} to `{}`",
                                                remote.remote_destination() // | &peer.b32
                                            ),
                                            Err(e) => warn!("{e}"),
                                        }
                                    }
                                }
                            });
                        }
                        Peer::Default(peer) => {
                            warn!(
                                "[tracker] unexpected peer `{peer}` from I2P tracker `{url}`, skip"
                            )
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

pub struct Buffer(pub Vec<Tracker>);

impl Buffer {
    /// Return peers from trackers
    pub async fn peers(&self, info_hash: &Id20) -> Result<HashSet<SocketAddr>> {
        let mut peers_b32 = HashSet::new(); // make sure I2P peers collected are unique as bind on different SocketAddr
        let mut peers = HashSet::new(); // unique peers buffer collected from all trackers

        for tracker in self.0.iter() {
            debug!(
                "[tracker] get peers from `{}` for `{}`...",
                tracker.url(),
                info_hash.as_string(),
            );
            peers.extend(tracker.peers(info_hash, &mut peers_b32).await?)
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
