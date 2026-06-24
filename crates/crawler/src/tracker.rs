use anyhow::{Result, bail};
use btpeer::Peer;
use librqbit::dht::Id20;
use log::*;
use std::{collections::HashSet, net::SocketAddr, time::Duration};
use url::Url;

struct Tracker {
    i2p_loopback: Option<SocketAddr>,
    proxy: Option<String>,
    timeout: Duration,
    url: Url,
}

impl Tracker {
    pub fn new(
        url: Url,
        timeout: u64,
        proxy: Option<String>,
        proxy_i2p: Option<String>,
        i2p_loopback: Option<SocketAddr>,
    ) -> Result<Self> {
        if !url.scheme().starts_with("http") {
            bail!("HTTP trackers only!")
        }
        Ok(Self {
            i2p_loopback,
            proxy: if url.host_str().unwrap().ends_with(".i2p") {
                if proxy_i2p.is_none() {
                    bail!("I2P proxy is required for tracker `{url}`")
                }
                if i2p_loopback.is_none() {
                    bail!("I2P loopback is required for tracker `{url}`")
                }
                info!(
                    "[tracker] init I2P tracker `{url}` using proxy {}",
                    proxy_i2p.as_ref().unwrap()
                );
                proxy_i2p
            } else {
                info!("[tracker] init tracker `{url}` using {proxy:?} proxy ");
                proxy
            },
            timeout: Duration::from_secs(timeout),
            url,
        })
    }

    /// Return resolved peers including local sockets over I2P/SAM (if exists)
    pub async fn peers(&self, info_hash: &Id20, announce_port: u16) -> Result<HashSet<SocketAddr>> {
        let announce =
            btpeer::http::query::Announce::new(self.url.as_str(), &info_hash.0, announce_port)?;

        let mut peers = HashSet::new();

        for p in if self.i2p_loopback.is_some() {
            btpeer::http::announce_i2p(&announce, self.timeout, self.proxy.as_deref())
                .await?
                .peers
                .0
        } else {
            btpeer::http::announce(&announce, self.timeout, self.proxy.as_deref())
                .await?
                .peers
                .0
        } {
            match p {
                Peer::Default(peer) => {
                    let p = SocketAddr::new(peer.host, peer.port);
                    if peers.insert(p) {
                        debug!("[tracker] add peer: `{p}`")
                    } else {
                        debug!("[tracker] replace existing peer: `{p}`")
                    }
                }
                Peer::I2p(peer) => {
                    // Create SAM bridge / local proxy as librqbit yet not supported I2P connections
                    use std::net::{IpAddr, Ipv4Addr, SocketAddr};
                    use yosemite::{Session, style::Stream};

                    let loopback = match self.i2p_loopback {
                        Some(l) => l,
                        None => {
                            let l = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0);
                            warn!(
                                "[tracker] returned I2P peer `{peer}` but I2P loopback address is not set; use defaults"
                            );
                            l
                        }
                    };
                    debug!("[tracker] init I2P loopback on `{loopback}`");

                    let listener = tokio::net::TcpListener::bind(loopback).await?;
                    let p = listener.local_addr()?;

                    if peers.insert(p) {
                        debug!("[tracker] add I2P peer: `{peer}` with SAM on `{p}`")
                    } else {
                        debug!("[tracker] replace I2P existing peer: `{peer}` with SAM on `{p}`")
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
                }
            }
        }
        Ok(peers)
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
            b.push(Tracker::new(
                url,
                timeout,
                proxy.as_ref().map(|p| p.to_string()),
                proxy_i2p.as_ref().map(|p| p.to_string()),
                loopback_i2p.copied(),
            )?)
        }
        Ok(Self(b))
    }

    /// Build magnet URI (`librqbit` impl dependency)
    pub fn magnet(&self, info_hash: &str) -> String {
        format!("magnet:?xt=urn:btih:{info_hash}")
    }

    /// Return resolved peers including local sockets over I2P/SAM (if exists)
    /// * optionally extends with `initial_peers` from argument
    pub async fn peers(
        &self,
        info_hash: &Id20,
        announce_port: u16,
        initial_peers: Option<&Vec<SocketAddr>>,
    ) -> Result<HashSet<SocketAddr>> {
        let mut peers = HashSet::new();
        for t in &self.0 {
            peers.extend(t.peers(info_hash, announce_port).await?);
        }
        if let Some(p) = initial_peers {
            peers.extend(p);
        }
        Ok(peers)
    }
}
