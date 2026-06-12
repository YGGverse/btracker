use anyhow::{Result, bail};
use btpeer::Peer;
use librqbit::dht::Id20;
use std::{collections::HashSet, net::SocketAddr, time::Duration};
use url::Url;

struct Tracker {
    // parse once
    is_i2p: bool,
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
    ) -> Result<Self> {
        if !url.scheme().starts_with("http") {
            bail!("HTTP trackers only!")
        }
        let is_i2p = url.host_str().unwrap().ends_with(".i2p");
        Ok(Self {
            is_i2p,
            proxy: if is_i2p {
                if proxy_i2p.is_none() {
                    bail!("I2P proxy is required for tracker `{url}`")
                }
                proxy_i2p
            } else {
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

        for p in if self.is_i2p {
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
                    peers.insert(SocketAddr::new(peer.host, peer.port));
                }
                Peer::I2p(peer) => {
                    use yosemite::{Session, style::Stream};

                    let loopback =
                        std::net::SocketAddrV4::new(std::net::Ipv4Addr::new(127, 0, 0, 1), 0);

                    let listener = tokio::net::TcpListener::bind(loopback).await?;
                    let proxy_address = listener.local_addr()?;
                    let mut session = Session::<Stream>::new(Default::default()).await?;

                    tokio::spawn(async move {
                        while let Ok((mut local, _)) = listener.accept().await {
                            if let Ok(mut remote) = session.connect(&peer.b32).await {
                                let _ =
                                    tokio::io::copy_bidirectional(&mut local, &mut remote).await;
                            }
                        }
                    });

                    peers.insert(proxy_address);
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
    ) -> Result<Self> {
        let mut b = Vec::with_capacity(trackers.len());
        for url in trackers {
            b.push(Tracker::new(
                url,
                timeout,
                proxy.as_ref().map(|p| p.to_string()),
                proxy_i2p.as_ref().map(|p| p.to_string()),
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
