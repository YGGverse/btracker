# yggtrackerd

![Linux](https://github.com/YGGverse/yggtrackerd/actions/workflows/linux.yml/badge.svg)
[![Dependencies](https://deps.rs/repo/github/YGGverse/yggtrackerd/status.svg)](https://deps.rs/repo/github/YGGverse/yggtrackerd)
[![crates.io](https://img.shields.io/crates/v/yggtrackerd.svg)](https://crates.io/crates/yggtrackerd)

BitTorrent aggregation web-server, based on the [Rocket](https://rocket.rs) framework and [aquatic-crawler](https://github.com/YGGverse/aquatic-crawler) FS

## Roadmap

* [x] RSS feeds
* [ ] Torrents listing
    * [x] Basic metainfo
    * [x] Pagination
    * [ ] Search filter
    * [ ] Results order
* [ ] Torrent details page
    * [ ] File list
    * [ ] Background image (from the files asset)
* [ ] Common features
    * [ ] Scrape peers/seeds
    * [ ] Download
        * [x] Magnet
        * [ ] Torrent

## Install

1. `git clone https://github.com/YGGverse/yggtrackerd.git && cd yggtrackerd`
2. `cargo build --release`
3. `sudo install target/release/yggtrackerd /usr/local/bin/yggtrackerd`

## Usage

``` bash
yggtrackerd --storage /path/to/aquatic-crawler/preload
```
* append `RUST_LOG=debug` for detailed information output

### Options

``` bash
yggtrackerd --help
```