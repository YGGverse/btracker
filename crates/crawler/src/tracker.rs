use anyhow::{Result, bail};
use btpeer::Peer;
use librqbit::dht::Id20;
use log::*;
use std::{
    collections::HashSet,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    time::Duration,
};
use url::Url;
use yosemite::{Session, style::Stream};

enum Tracker {
    Default {
        proxy: Option<String>,
        timeout: Duration,
        url: Url,
    },
    I2p {
        loopback: IpAddr,
        proxy: String,
        timeout: Duration,
        url: Url,
        inbound_len: usize,
        outbound_len: usize,
    },
}

impl Tracker {
    /// Construct default Tracker
    fn default(url: Url, timeout: u64, proxy: Option<String>) -> Result<Self> {
        if !url.scheme().starts_with("http") {
            bail!("HTTP trackers only!")
        }
        if is_i2p(&url) {
            bail!("Unexpected constructor for I2P tracker!")
        }
        info!("[tracker] init default tracker `{url}` using {proxy:?} proxy");
        Ok(Self::Default {
            proxy,
            timeout: Duration::from_secs(timeout),
            url,
        })
    }

    /// Construct I2P tracker
    fn i2p(
        url: Url,
        timeout: u64,
        proxy: String,

        inbound_len: usize,
        outbound_len: usize,
        loopback: Option<IpAddr>,
    ) -> Result<Self> {
        if !url.scheme().starts_with("http") {
            bail!("HTTP trackers only!")
        }
        if !is_i2p(&url) {
            bail!("Unexpected constructor for default tracker!")
        }
        info!("[tracker] init I2P tracker `{url}` using proxy `{proxy}`");
        Ok(Self::I2p {
            loopback: match loopback {
                Some(ip) => {
                    debug!("[tracker] loopback address is `{ip}`");
                    ip
                }
                None => {
                    let ip = IpAddr::V4(Ipv4Addr::LOCALHOST);
                    warn!("[tracker] loopback address is not configured; use default `{ip}`");
                    ip
                }
            },
            proxy,
            timeout: Duration::from_secs(timeout),
            url,
            inbound_len,
            outbound_len,
        })
    }

    /// Get peers by `info_hash` from Trackers
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
                        Peer::Default(peer) => {
                            let p = SocketAddr::new(peer.host, peer.port);
                            if b.insert(p) {
                                debug!("[tracker] add peer: `{p}`")
                            } else {
                                debug!("[tracker] replace existing peer: `{p}`")
                            }
                        }
                        Peer::I2p(peer) => {
                            warn!(
                                "[tracker] unexpected I2P peer `{peer}` from default tracker `{url}`, skip"
                            )
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
                    btpeer::http::announce_i2p(&announce, *timeout, Some(proxy))
                        .await?
                        .peers
                        .0,
                    peers_limit_per_tracker_i2p,
                );

                let mut b = HashSet::with_capacity(peers.len());

                for p in peers {
                    match p {
                        // Create SAM bridge / local proxy as librqbit yet not supported I2P connections
                        Peer::I2p(peer) => {
                            if !peers_b32.insert(peer.b32.clone()) {
                                debug!(
                                    "[tracker] b32 value `{}` for peer `{peer}` on `{loopback}` exists, skip.",
                                    &peer.b32
                                );
                                continue;
                            }

                            debug!("[tracker] bind proxy listener for `{peer}` on `{loopback}`...");

                            let listener =
                                tokio::net::TcpListener::bind(SocketAddr::new(*loopback, 0))
                                    .await?;

                            let p = listener.local_addr()?;

                            if b.insert(p) {
                                debug!(
                                    "[tracker] bind I2P peer `{peer}` as `{p}`; init SAM session..."
                                )
                            } else {
                                debug!(
                                    "[tracker] bind existing I2P peer `{peer}` as `{p}`; init SAM session..."
                                )
                            }

                            let mut session = Session::<Stream>::new(yosemite::SessionOptions {
                                inbound_len: *inbound_len,
                                outbound_len: *outbound_len,
                                ..yosemite::SessionOptions::default()
                            })
                            .await?;

                            debug!(
                                "[tracker] listening incoming connections for `{peer}` on `{p}`..."
                            );

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
                                        match tokio::io::copy_bidirectional(&mut local, &mut remote)
                                            .await
                                        {
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
                        }
                        Peer::Default(peer) => warn!(
                            "[tracker] unexpected default peer `{peer}` from I2P tracker `{url}`, skip"
                        ),
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

pub struct Buffer(Vec<Tracker>);

impl Buffer {
    /// Construct new buffer
    pub fn new(
        trackers: Vec<Url>,
        timeout: u64,
        proxy: Option<&Url>,
        proxy_i2p: Option<&Url>,
        loopback_i2p: Option<&IpAddr>,
        (inbound_len, outbound_len): (usize, usize),
    ) -> Result<Self> {
        let mut b = Vec::with_capacity(trackers.len());
        for url in trackers {
            b.push(if is_i2p(&url) {
                Tracker::i2p(
                    url,
                    timeout,
                    match proxy_i2p {
                        Some(p) => p.to_string(),
                        None => {
                            bail!("[tracker] found I2P tracker but its proxy was not configured")
                        }
                    },
                    inbound_len,
                    outbound_len,
                    loopback_i2p.copied(),
                )?
            } else {
                Tracker::default(url, timeout, proxy.as_ref().map(|p| p.to_string()))?
            })
        }
        Ok(Self(b))
    }

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

fn is_i2p(url: &Url) -> bool {
    url.host_str().is_some_and(|h| h.ends_with(".i2p"))
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
