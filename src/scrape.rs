#[derive(rocket::serde::Serialize, Default)]
#[serde(crate = "rocket::serde")]
pub struct Result {
    pub leechers: u32,
    pub peers: u32,
    pub seeders: u32,
}

pub fn get(scrape: &super::Scrape, id: [u8; 20]) -> Option<Result> {
    scrape.get(id).map(|s| Result {
        leechers: s.leechers,
        peers: s.peers,
        seeders: s.seeders,
    })
}
