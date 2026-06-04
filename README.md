# btracker

![Linux](https://github.com/yggverse/btracker/actions/workflows/linux.yml/badge.svg)

βtracker is an ecosystem to deploy BitTorrent open tracker with DHT search engine.

## Install

``` bash
git clone https://github.com/YGGverse/btracker.git
cd btracker
cargo build --release -p btracker-http
sudo install target/release/btracker-http /usr/local/bin
```
* to setup Rust environment see [rustup](https://rustup.rs)

> [!TIP]
>
> See working server [config examples](https://codeberg.org/YGGverse/server/search/branch/main?path=&q=btracker) to deploy