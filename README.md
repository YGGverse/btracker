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
    * [ ] Scrape peers/seeds
    * [ ] Download
        * [x] Magnet
        * [ ] Torrent

## Install

1. `git clone https://github.com/yggverse/btracker.git && cd btracker`
2. `cargo build --release`
3. `sudo install target/release/btracker /usr/local/bin/btracker`

## Usage

``` bash
btracker --storage /path/to/aquatic-crawler/preload
```
* append `RUST_LOG=debug` for detailed information output
* see the project [Wiki](https://github.com/YGGverse/btracker/wiki) for more details (including [systemd](https://github.com/YGGverse/btracker/wiki/Systemd) and [nginx](https://github.com/YGGverse/btracker/wiki/Nginx) examples)

### Options

``` bash
btracker --help
```

## Live

* `http://[302:68d0:f0d5:b88d::fdb]` - [Yggdrasil](https://yggdrasil-network.github.io/) only peers BitTorrent tracker
    * [http://tracker.ygg](http://tracker.ygg/) - [Alfis DNS](https://github.com/Revertron/Alfis) alias