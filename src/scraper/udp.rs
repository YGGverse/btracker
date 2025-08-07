use super::Scrape;
use librqbit_core::hash_id::Id20;
use rand::Rng;
use std::{
    io::Error,
    net::{SocketAddr, UdpSocket},
    time::Duration,
};

struct Route {
    socket: UdpSocket,
    remote: Vec<SocketAddr>,
}

pub struct Udp(Vec<Route>);

impl Udp {
    pub fn init(local: Vec<SocketAddr>, remote: Vec<SocketAddr>) -> Self {
        Self(
            local
                .into_iter()
                .map(|l| {
                    let socket = UdpSocket::bind(l).unwrap();
                    socket
                        .set_read_timeout(Some(Duration::from_secs(3)))
                        .unwrap();
                    Route {
                        socket,
                        remote: if l.is_ipv4() {
                            remote.iter().filter(|r| r.is_ipv4()).cloned().collect()
                        } else {
                            remote.iter().filter(|r| r.is_ipv6()).cloned().collect()
                        },
                    }
                })
                .collect(),
        )
    }

    pub fn scrape(&self, info_hash: Id20) -> Result<Scrape, Error> {
        let mut t = Scrape::default();
        for route in &self.0 {
            for remote in &route.remote {
                route.socket.send_to(&connection_request(), remote)?;

                let mut b = [0u8; 16];
                if route.socket.recv(&mut b)? < 16 {
                    todo!()
                }
                route.socket.send_to(
                    &scrape_request(
                        u64::from_be_bytes(b[8..16].try_into().unwrap()),
                        rand::rng().random::<u32>(),
                        &[info_hash],
                    ),
                    remote,
                )?;

                let mut b = [0u8; 1024];
                let l = route.socket.recv(&mut b)?;
                if l < 20 {
                    todo!()
                }

                t.seeders += u32::from_be_bytes(b[8..12].try_into().unwrap());
                t.leechers += u32::from_be_bytes(b[12..16].try_into().unwrap());
                t.peers += u32::from_be_bytes(b[16..20].try_into().unwrap());
            }
        }
        Ok(t)
    }
}

fn connection_request() -> Vec<u8> {
    let mut b = Vec::new();
    b.extend_from_slice(&0x41727101980u64.to_be_bytes());
    b.extend_from_slice(&0u32.to_be_bytes());
    b.extend_from_slice(&rand::rng().random::<u32>().to_be_bytes());
    b
}

fn scrape_request(connection_id: u64, transaction_id: u32, info_hashes: &[Id20]) -> Vec<u8> {
    let mut b = Vec::new();
    b.extend_from_slice(&connection_id.to_be_bytes());
    b.extend_from_slice(&2u32.to_be_bytes());
    b.extend_from_slice(&transaction_id.to_be_bytes());
    // * up to about 74 torrents can be scraped at once
    //   https://www.bittorrent.org/beps/bep_0015.html
    if info_hashes.len() > 74 {
        todo!()
    }
    for hash in info_hashes {
        b.extend_from_slice(&hash.0);
    }
    b
}
