use std::{net::SocketAddr, str::FromStr};

pub struct Peers(Vec<SocketAddr>);

impl Peers {
    pub fn init(peers: &Vec<String>) -> anyhow::Result<Self> {
        let mut p = Vec::with_capacity(peers.len());
        for peer in peers {
            p.push(SocketAddr::from_str(peer)?);
        }
        Ok(Self(p))
    }

    pub fn initial_peers(&self) -> Option<Vec<SocketAddr>> {
        if self.0.is_empty() {
            None
        } else {
            Some(self.0.clone())
        }
    }
}
