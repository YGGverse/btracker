use anyhow::{Result, bail};
use btpeer::udp::Server;
use std::{
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6, UdpSocket},
    str::FromStr,
    time::Duration,
};
use url::Url;

#[derive(Default)]
pub struct Total {
    pub complete: u32,
    pub downloaded: u32,
    pub incomplete: u32,
}

enum Scrape {
    Http {
        headers: Vec<String>,
        proxy: Option<String>,
        timeout: Duration,
        tracker: Url,
    },
    Udp {
        server: UdpSocket,
        tracker: SocketAddr,
    },
}

impl Scrape {
    pub fn new(
        tracker: Url,
        timeout: u64,
        headers: Vec<String>,
        proxy: Option<String>,
        proxy_i2p: Option<String>,
    ) -> Result<Self> {
        if tracker.host_str().is_none() {
            bail!("Tracker host is required")
        }

        let host = tracker.host_str().unwrap();

        Ok(if tracker.scheme().starts_with("http") {
            Self::Http {
                proxy: if host.ends_with(".i2p") {
                    if proxy_i2p.is_none() {
                        bail!("I2P proxy is required for tracker `{tracker}`")
                    }
                    proxy_i2p
                } else {
                    proxy
                },
                timeout: Duration::from_secs(timeout),
                tracker,
                headers,
            }
        } else if tracker.scheme() == "udp" {
            if host.ends_with(".i2p") {
                bail!("I2P supports HTTP trackers only")
            }

            let ip = IpAddr::from_str(host.trim_start_matches('[').trim_end_matches(']'))?; // @TODO DNS for UDP trackers yet not supported
            let timeout = Duration::from_secs(timeout);

            Self::Udp {
                server: {
                    let socket = if ip.is_ipv6() {
                        UdpSocket::bind(SocketAddr::V6(SocketAddrV6::new(
                            Ipv6Addr::UNSPECIFIED,
                            0,
                            0,
                            0,
                        )))?
                    } else {
                        UdpSocket::bind(SocketAddr::V4(SocketAddrV4::new(
                            Ipv4Addr::UNSPECIFIED,
                            0,
                        )))?
                    };
                    socket.set_read_timeout(Some(timeout))?;
                    socket.set_write_timeout(Some(timeout))?;
                    socket
                },
                tracker: SocketAddr::from_str(&format!(
                    "{}:{}",
                    if ip.is_ipv6() {
                        format!("[{ip}]")
                    } else {
                        ip.to_string()
                    },
                    tracker.port().unwrap()
                ))?,
            }
        } else {
            bail!("Unsupported tracker scheme")
        })
    }

    pub async fn get(&self, id20: &[[u8; 20]]) -> Result<Total> {
        Ok(match self {
            Self::Http {
                headers,
                proxy,
                timeout,
                tracker,
            } => {
                let response = btpeer::http::scrape(
                    &btpeer::http::query::Scrape::new(tracker.as_str(), Some(id20))?,
                    *timeout,
                    headers,
                    proxy.as_deref(),
                )
                .await?;
                Total {
                    complete: response.total.complete,
                    downloaded: response.total.downloaded,
                    incomplete: response.total.incomplete,
                }
            }
            Self::Udp { server, tracker } => {
                let response = server.scrape(tracker, id20)?;
                Total {
                    complete: response.seeders,
                    downloaded: response.peers,
                    incomplete: response.leechers,
                }
            }
        })
    }
}

pub struct Buffer(Vec<Scrape>);

impl Buffer {
    pub fn new(
        trackers: Vec<Url>,
        timeout: u64,
        proxy: Option<&Url>,
        proxy_i2p: Option<&Url>,
        headers: &[String],
    ) -> Result<Self> {
        let mut this = Vec::with_capacity(trackers.len());

        for url in trackers {
            this.push(Scrape::new(
                url,
                timeout,
                headers.to_owned(),
                proxy.as_ref().map(|p| p.to_string()),
                proxy_i2p.as_ref().map(|p| p.to_string()),
            )?)
        }

        Ok(Self(this))
    }

    pub async fn get(&self, id20: &[[u8; 20]]) -> Result<Total> {
        let mut total = Total::default();

        for this in self.0.iter() {
            let result = this.get(id20).await?;
            total.complete += result.complete;
            total.downloaded += result.downloaded;
            total.incomplete += result.incomplete;
        }

        Ok(total)
    }
}
