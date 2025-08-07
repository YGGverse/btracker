# btracker

![Linux](https://github.com/yggverse/btracker/actions/workflows/linux.yml/badge.svg)
[![Dependencies](https://deps.rs/repo/github/yggverse/btracker/status.svg)](https://deps.rs/repo/github/yggverse/btracker)
[![crates.io](https://img.shields.io/crates/v/btracker.svg)](https://crates.io/crates/btracker)

βtracker is a BitTorrent aggregator based on the [aquatic-crawler](https://github.com/yggverse/aquatic-crawler) API and [Rocket](https://rocket.rs) web-framework

## Roadmap

* [x] RSS feeds
* [ ] Torrents listing
    * [x] Basic metainfo
    * [x] Pagination
    * [ ] Search filter
    * [ ] Results order
* [ ] Torrent details page
    * [ ] Files list
    * [ ] Background image (from the files asset)
* [ ] Common features
    * [ ] Scrape peers/seeders/leechers
        * [x] UDP
        * [ ] TCP
    * [ ] Download
        * [x] Magnet
        * [ ] Torrent

## Install

1. `git clone https://github.com/yggverse/btracker.git && cd btracker`
2. `cargo build --release`
3. `sudo install target/release/btracker /usr/local/bin/btracker`

## Usage

``` bash
btracker --preload=/path/to/aquatic-crawler/preload\
         --scrape=udp://127.0.0.1:6969\
         --tracker=udp://[302:68d0:f0d5:b88d::fdb]:6969\
         --tracker=udp://tracker.ygg:6969
```
* The `--preload` argument specifies the location of the crawled torrents (see [aquatic-crawler](https://github.com/yggverse/aquatic-crawler))
* The `--scrape` argument is optional and enables statistics for peers, seeders, and leechers
  * it is recommended to use the local address for faster performance
  * this argument supports multiple definitions for both the IPv4 and IPv6 protocols, parsed from the URL value
  * take a look at the `--udp` option if you want to customize the default binding for UDP scrapes
* Define as many `--tracker`(s) as required
* Append `RUST_LOG=debug` for detailed information output; use `--debug` to configure as `rocket::Config::debug_default()`
* See the project [Wiki](https://github.com/YGGverse/btracker/wiki) for more details (including [systemd](https://github.com/YGGverse/btracker/wiki/Systemd) and [nginx](https://github.com/YGGverse/btracker/wiki/Nginx) examples)

### Options

``` bash
btracker --help
```

## Live

* `http://[302:68d0:f0d5:b88d::fdb]` - [Yggdrasil](https://yggdrasil-network.github.io/) only peers BitTorrent tracker
    * [http://tracker.ygg](http://tracker.ygg/) - [Alfis DNS](https://github.com/Revertron/Alfis) alias