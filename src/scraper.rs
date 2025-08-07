mod udp;

use rocket::serde::Serialize;
use std::net::SocketAddr;
use udp::Udp;

#[derive(Serialize, Default)]
#[serde(crate = "rocket::serde")]
pub struct Scrape {
    pub leechers: u32,
    pub peers: u32,
    pub seeders: u32,
}

pub struct Scraper {
    udp: Option<Udp>,
    // tcp: @TODO
}

impl Scraper {
    pub fn init(udp: Option<(Vec<SocketAddr>, Vec<SocketAddr>)>) -> Self {
        Self {
            udp: udp.map(|(local, remote)| Udp::init(local, remote)),
        }
    }

    pub fn scrape(&self, info_hash: &[u8]) -> Option<Scrape> {
        self.udp.as_ref()?;
        let mut t = Scrape::default();
        if let Some(ref u) = self.udp {
            let r = u.scrape(info_hash).ok()?; // @TODO handle
            t.leechers += r.leechers;
            t.peers += r.peers;
            t.seeders += r.seeders;
        }
        Some(t)
    }
}
