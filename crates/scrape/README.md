# btracker-scrape

[![Dependencies](https://deps.rs/repo/github/YGGverse/btracker-scrape/status.svg)](https://deps.rs/repo/github/YGGverse/btracker-scrape)
[![crates.io](https://img.shields.io/crates/v/btracker-scrape.svg)](https://crates.io/crates/btracker-scrape)

Shared BitTorrent scrape API for the βtracker project components

## Install

``` bash
cargo add btracker-scrape
```

## Usage

``` rust
let udp_server = vec![
    SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 0)),
    SocketAddr::V6(SocketAddrV6::new(Ipv6Addr::UNSPECIFIED, 0, 0, 0))
];

let udp_trackers = vec![
    SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 6969))
];

let scrape = Scrape::init(Some(udp_server, udp_trackers));

println!("{:?}", scrape.get([u8; 20])); // hash v1
```