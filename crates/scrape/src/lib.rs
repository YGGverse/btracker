mod udp;

use std::net::SocketAddr;
use udp::Udp;

#[derive(Default)]
pub struct Result {
    pub leechers: u32,
    pub peers: u32,
    pub seeders: u32,
}

pub struct Scrape {
    udp: Option<Udp>,
    // tcp: @TODO
}

impl Scrape {
    pub fn init(udp: Option<(Vec<SocketAddr>, Vec<SocketAddr>)>) -> Self {
        Self {
            udp: udp.map(|(local, remote)| Udp::init(local, remote)),
        }
    }

    pub fn get(&self, info_hash: [u8; 20]) -> Option<Result> {
        self.udp.as_ref()?;
        let mut t = Result::default();
        if let Some(ref u) = self.udp {
            let r = u.scrape(info_hash).ok()?; // @TODO handle
            t.leechers += r.leechers;
            t.peers += r.peers;
            t.seeders += r.seeders;
        }
        Some(t)
    }
}
