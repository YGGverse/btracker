# btracker

![Linux](https://github.com/yggverse/btracker/actions/workflows/linux.yml/badge.svg)
[![Dependencies](https://deps.rs/repo/github/yggverse/btracker/status.svg)](https://deps.rs/repo/github/yggverse/btracker)
[![crates.io](https://img.shields.io/crates/v/btracker.svg)](https://crates.io/crates/btracker)

βtracker is a social BitTorrent aggregator based on the [aquatic-crawler](https://github.com/yggverse/aquatic-crawler) FS API and [Rocket](https://rocket.rs) web-framework.

## Screenshots

See the [Wiki](https://github.com/YGGverse/btracker/wiki/Screenshots) page

## Live

* `http://[302:68d0:f0d5:b88d::fdb]` - [Yggdrasil](https://yggdrasil-network.github.io/) only peers BitTorrent tracker
    * [http://tracker.ygg](http://tracker.ygg/) - [Alfis DNS](https://github.com/Revertron/Alfis) alias

## Roadmap

* [ ] Listing (index) page
    * [x] Basic metainfo
    * [x] Pagination
    * [x] Search
        * [x] multiple keyword support
            [ ] configurable split separators
        * [x] torrent meta match
            * [x] name
            * [x] comment
            * [x] created by
            * [x] publisher
            * [x] publisher URL
            * [x] announce
            * [x] announce list
            * [x] file names
        * [ ] relevance ranking
        * [ ] fast n-memory index
        * [ ] search options form
    * [ ] results order controls (torrent indexed by default)
* [x] Details page
    * [x] files
        * [x] clickable content preview
    * [x] name
    * [x] comment
    * [x] created at
    * [ ] created by
    * [ ] publisher
    * [ ] publisher URL
    * [ ] announce
    * [ ] announce list
* [ ] Common features
    * [ ] scrape peers/seeders/leechers
        * [x] UDP
        * [ ] TCP
    * [ ] download
        * [x] magnet link
        * [ ] torrent file
            * [x] from the `public` location
            * [ ] filtered trackers binary
* [x] RSS feed

## Install

### Stable

``` bash
cargo install btracker
```

### Repository

1. `git clone https://github.com/yggverse/btracker.git && cd btracker`
2. `cargo build --release`
3. `sudo install target/release/btracker /usr/local/bin/btracker`
    * copy `public` & `templates` folders to the [server destination](https://rocket.rs/guide/v0.5/deploying/)

## Usage

``` bash
btracker --public=/path/to/aquatic-crawler/preload\
         --scrape=udp://127.0.0.1:6969\
         --tracker=udp://[302:68d0:f0d5:b88d::fdb]:6969\
         --tracker=udp://tracker.ygg:6969
```
* The `--public` argument specifies the location of the crawled torrents (see [aquatic-crawler](https://github.com/yggverse/aquatic-crawler))
    * make sure this location also contains a copy (or symlink) of the `/public` files from this crate (see the [Rocket deploying specification](https://rocket.rs/guide/v0.5/deploying/))
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
