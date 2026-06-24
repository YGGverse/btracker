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
        loopback: SocketAddr,
        proxy: String,
        timeout: Duration,
        url: Url,
    },
}

impl Tracker {
    pub fn default(url: Url, timeout: u64, proxy: Option<String>) -> Result<Self> {
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

    pub fn i2p(
        url: Url,
        timeout: u64,
        proxy: String,
        loopback: Option<SocketAddr>,
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
                Some(l) => l,
                None => {
                    let l = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0);
                    warn!("[tracker] custom loopback address is not set; use default `{l}`");
                    l
                }
            },
            proxy,
            timeout: Duration::from_secs(timeout),
            url,
        })
    }

    pub async fn peers(&self, info_hash: &Id20, announce_port: u16) -> Result<HashSet<SocketAddr>> {
        Ok(match self {
            Self::Default {
                proxy,
                timeout,
                url,
            } => {
                let announce =
                    btpeer::http::query::Announce::new(url.as_str(), &info_hash.0, announce_port)?;

                let peers = btpeer::http::announce(&announce, *timeout, proxy.as_deref())
                    .await?
                    .peers
                    .0;

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
                loopback,
                proxy,
                timeout,
                url,
            } => {
                let announce =
                    btpeer::http::query::Announce::new(url.as_str(), &info_hash.0, announce_port)?;

                let peers = btpeer::http::announce_i2p(&announce, *timeout, Some(proxy))
                    .await?
                    .peers
                    .0;

                let mut b = HashSet::with_capacity(peers.len());

                for p in peers {
                    match p {
                        // Create SAM bridge / local proxy as librqbit yet not supported I2P connections
                        Peer::I2p(peer) => {
                            debug!("[tracker] init I2P loopback on `{loopback}`");

                            let listener = tokio::net::TcpListener::bind(loopback).await?;
                            let p = listener.local_addr()?;

                            if b.insert(p) {
                                debug!("[tracker] add I2P peer: `{peer}` with SAM on `{p}`")
                            } else {
                                debug!(
                                    "[tracker] replace I2P existing peer: `{peer}` with SAM on `{p}`"
                                )
                            }

                            let mut session = Session::<Stream>::new(Default::default()).await?;

                            tokio::spawn(async move {
                                while let Ok((mut local, _)) = listener.accept().await {
                                    debug!(
                                        "[tracker] accepting SAM connection from {:?} ({})",
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
}

pub struct Buffer(Vec<Tracker>);

impl Buffer {
    pub fn new(
        trackers: Vec<Url>,
        timeout: u64,
        proxy: Option<&Url>,
        proxy_i2p: Option<&Url>,
        loopback_i2p: Option<&SocketAddr>,
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
                    loopback_i2p.copied(),
                )?
            } else {
                Tracker::default(url, timeout, proxy.as_ref().map(|p| p.to_string()))?
            })
        }
        Ok(Self(b))
    }

    /// Build magnet URI (`librqbit` impl dependency)
    pub fn magnet(&self, info_hash: &str) -> String {
        format!("magnet:?xt=urn:btih:{info_hash}")
    }

    /// Return resolved peers from default trackers
    /// * optionally extend with `initial_peers`
    pub async fn peers(
        &self,
        info_hash: &Id20,
        announce_port: u16,
        initial_peers: Option<&Vec<SocketAddr>>,
    ) -> Result<HashSet<SocketAddr>> {
        let mut peers = HashSet::new();
        for t in self
            .0
            .iter()
            .filter(|t| matches!(t, Tracker::Default { .. }))
        {
            peers.extend(t.peers(info_hash, announce_port).await?);
        }
        if let Some(p) = initial_peers {
            peers.extend(p);
        }
        Ok(peers)
    }

    /// Return resolved peers from I2P trackers
    /// * optionally extend with `initial_peers`
    pub async fn peers_i2p(
        &self,
        info_hash: &Id20,
        announce_port: u16,
        initial_peers: Option<&Vec<SocketAddr>>,
    ) -> Result<HashSet<SocketAddr>> {
        let mut peers = HashSet::new();
        for t in self.0.iter().filter(|t| matches!(t, Tracker::I2p { .. })) {
            peers.extend(t.peers(info_hash, announce_port).await?);
        }
        if let Some(p) = initial_peers {
            peers.extend(p);
        }
        Ok(peers)
    }
}

fn is_i2p(url: &Url) -> bool {
    url.host_str().unwrap().ends_with(".i2p")
}
