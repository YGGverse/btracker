# btracker-gemini

[![Dependencies](https://deps.rs/repo/github/YGGverse/btracker-gemini/status.svg)](https://deps.rs/repo/github/YGGverse/btracker-gemini)
[![crates.io](https://img.shields.io/crates/v/btracker-gemini.svg)](https://crates.io/crates/btracker-gemini)

βtracker server implementation for the [Gemini protocol](http://geminiprotocol.net)

> [!NOTE]
> In development!

## Install

``` bash
git clone https://github.com/YGGverse/btracker.git && cd btracker
cargo build --release -p btracker-gemini
sudo install target/release/btracker-gemini /usr/local/bin
```
* to setup Rust environment see [rustup](https://rustup.rs)

## Setup

<details>
<summary>Generate PKCS (PFX)</summary>
<pre>
openssl genpkey -algorithm RSA -out server.pem -pkeyopt rsa_keygen_bits:2048
openssl req -new -key server.pem -out request.csr
openssl x509 -req -in request.csr -signkey server.pem -out server.crt -days 365
openssl pkcs12 -export -out server.pfx -inkey server.pem -in server.crt</pre>
</details>

## Launch

``` bash
btracker-gemini -i /path/to/server.pfx\
                -s /path/to/btracker-fs\
                -t udp://tracker1:6969\
                -t udp://tracker2:6969
```
* prepend `RUST_LOG=trace` or `RUST_LOG=btracker_gemini=trace` to debug
* use `-b` to bind server on specified `host:port`
* use `-h` to print all available options