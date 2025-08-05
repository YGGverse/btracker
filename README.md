# btracker

![Linux](https://github.com/YGGverse/btracker/actions/workflows/linux.yml/badge.svg)
[![Dependencies](https://deps.rs/repo/github/YGGverse/btracker/status.svg)](https://deps.rs/repo/github/YGGverse/btracker)
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

### Options

``` bash
btracker --help
```