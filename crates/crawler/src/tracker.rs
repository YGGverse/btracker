use anyhow::Result;
use btpeer::Peer;
use librqbit::dht::Id20;
use log::*;
use std::{
    collections::HashSet,
    net::{IpAddr, SocketAddr},
    time::Duration,
};
use url::Url;
use yosemite::{Session, style::Stream};

pub enum Tracker {
    Default {
        proxy: Option<String>,
        timeout: Duration,
        url: Url,
    },
    I2p {
        loopback: IpAddr,
        proxy: Option<String>,
        timeout: Duration,
        url: Url,
        inbound_len: usize,
        outbound_len: usize,
    },
}

impl Tracker {
    async fn peers(
        &self,
        info_hash: &Id20,
        announce_port: u16,
        peers_limit_per_tracker: Option<usize>,
        peers_limit_per_tracker_i2p: Option<usize>,
        peers_b32: &mut HashSet<String>,
    ) -> Result<HashSet<SocketAddr>> {
        Ok(match self {
            Self::Default {
                proxy,
                timeout,
                url,
            } => {
                let announce =
                    btpeer::http::query::Announce::new(url.as_str(), &info_hash.0, announce_port)?;

                let peers = take_random_peers(
                    btpeer::http::announce(&announce, *timeout, proxy.as_deref())
                        .await?
                        .peers
                        .0,
                    peers_limit_per_tracker,
                );

                let mut b = HashSet::with_capacity(peers.len());

                for p in peers {
                    match p {
                        Peer::Default(peer) => handle_default_peer(&mut b, peer),
                        Peer::I2p(peer) => {
                            warn!("[tracker] unexpected peer `{peer}` from tracker `{url}`, skip")
                        }
                    }
                }
                b
            }
            Self::I2p {
                inbound_len,
                outbound_len,
                loopback,
                proxy,
                timeout,
                url,
            } => {
                let announce =
                    btpeer::http::query::Announce::new(url.as_str(), &info_hash.0, announce_port)?;

                let peers = take_random_peers(
                    btpeer::http::announce_i2p(&announce, *timeout, proxy.as_deref())
                        .await?
                        .peers
                        .0,
                    peers_limit_per_tracker_i2p,
                );

                let mut b = HashSet::with_capacity(peers.len());

                for p in peers {
                    match p {
                        Peer::I2p(peer) => {
                            handle_i2p_peer(
                                &mut b,
                                peers_b32,
                                peer,
                                *loopback,
                                *inbound_len,
                                *outbound_len,
                            )
                            .await?
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
    pub async fn peers(
        &self,
        info_hash: &Id20,
        announce_port: u16,
        peers_limit_per_tracker: Option<usize>,
        peers_limit_per_tracker_i2p: Option<usize>,
        force_extend_with_peers: Option<&Vec<SocketAddr>>,
    ) -> Result<HashSet<SocketAddr>> {
        let mut peers_b32 = HashSet::new(); // make sure I2P peers collected are unique as bind on different SocketAddr
        let mut peers = HashSet::new(); // unique peers buffer collected from all trackers

        for tracker in self.0.iter() {
            debug!(
                "[tracker] get peers from `{}` for `{}`...",
                info_hash.as_string(),
                tracker.url()
            );
            peers.extend(
                tracker
                    .peers(
                        info_hash,
                        announce_port,
                        peers_limit_per_tracker,
                        peers_limit_per_tracker_i2p,
                        &mut peers_b32,
                    )
                    .await?,
            )
        }

        if let Some(p) = force_extend_with_peers {
            debug!("[tracker] forcefully extend with {} peers ({p:?})", p.len());
            peers.extend(p);
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
                "[tracker] take {}/{total} random peers as limited to {l}",
                p.len()
            );
            p
        }
        None => {
            debug!("[tracker] take all {total} peers");
            peers
        }
    }
}

fn handle_default_peer(peers: &mut HashSet<SocketAddr>, peer: btpeer::peer::Default) {
    let p = SocketAddr::new(peer.host, peer.port);
    if peers.insert(p) {
        debug!("[tracker] add peer: `{p}`")
    } else {
        debug!("[tracker] replace existing peer: `{p}`")
    }
}

/// Create SAM bridge / local proxy as librqbit yet not supported I2P connections
async fn handle_i2p_peer(
    peers: &mut HashSet<SocketAddr>,
    peers_b32: &mut HashSet<String>,
    peer: btpeer::peer::I2p,
    loopback: IpAddr,
    inbound_len: usize,
    outbound_len: usize,
) -> Result<()> {
    if !peers_b32.insert(peer.b32.clone()) {
        debug!(
            "[tracker] b32 value `{}` for peer `{peer}` on `{loopback}` exists, skip.",
            &peer.b32
        );
        return Ok(());
    }

    debug!("[tracker] bind proxy listener for `{peer}` on `{loopback}`...");

    let listener = tokio::net::TcpListener::bind(SocketAddr::new(loopback, 0)).await?;

    let p = listener.local_addr()?;

    if peers.insert(p) {
        debug!("[tracker] bind I2P peer `{peer}` as `{p}`; init SAM session...")
    } else {
        debug!("[tracker] bind existing I2P peer `{peer}` as `{p}`; init SAM session...")
    }

    let mut session = Session::<Stream>::new(yosemite::SessionOptions {
        inbound_len,
        outbound_len,
        ..yosemite::SessionOptions::default()
    })
    .await?;

    debug!("[tracker] listening incoming connections for `{peer}` on `{p}`...");

    tokio::spawn(async move {
        while let Ok((mut local, client)) = listener.accept().await {
            debug!(
                "[tracker] accepting SAM connection from {client} to {:?} ({})",
                local.peer_addr(),
                &peer.b32
            );
            if let Ok(mut remote) = session.connect(&peer.b32).await {
                debug!(
                    "[tracker] begin SAM connection to `{}` ({})",
                    remote.remote_destination(),
                    &peer.b32
                );
                match tokio::io::copy_bidirectional(&mut local, &mut remote).await {
                    Ok((a, b)) => trace!(
                        "[tracker] copied {a}/{b} to `{}` ({})",
                        remote.remote_destination(),
                        &peer.b32
                    ),
                    Err(e) => warn!("{e}"),
                }
            }
        }
    });

    Ok(())
}
