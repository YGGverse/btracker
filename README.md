# yggtrackerd

![Linux](https://github.com/YGGverse/yggtrackerd/actions/workflows/linux.yml/badge.svg)
[![Dependencies](https://deps.rs/repo/github/YGGverse/yggtrackerd/status.svg)](https://deps.rs/repo/github/YGGverse/yggtrackerd)
[![crates.io](https://img.shields.io/crates/v/yggtrackerd.svg)](https://crates.io/crates/yggtrackerd)

Crawler daemon for the yggtracker-redb index, based on the librqbit API

## Install

1. `git clone https://github.com/YGGverse/yggtrackerd.git && cd yggtrackerd`
2. `cargo build --release`
3. `sudo install target/release/yggtrackerd /usr/local/bin/yggtrackerd`

## Usage

``` bash
yggtrackerd --infohash /path/to/info-hash-ipv6.bin\
            --infohash /path/to/another-source.bin\
            --tracker  udp://host1:port\
            --tracker  udp://host2:port\
            --database /path/to/index.redb\
            --preload  /path/to/directory
```

### Options

``` bash
yggtrackerd --help
```