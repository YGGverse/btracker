# btracker-crawler

[![Dependencies](https://deps.rs/repo/github/YGGverse/btracker-crawler/status.svg)](https://deps.rs/repo/github/YGGverse/btracker-crawler)
[![crates.io](https://img.shields.io/crates/v/btracker-crawler.svg)](https://crates.io/crates/btracker-crawler)

SSD-friendly FS crawler of BEP 48 / Full Scrape, based on the [librqbit](https://github.com/ikatson/rqbit/tree/main/crates/librqbit) resolver

> [!NOTE]
> * By using OpenTracker as the index source, please make sure `FEATURES+=-DWANT_FULLSCRAPE` is enabled!
> * Enable experimental I2P support by `cargo build --features i2p`