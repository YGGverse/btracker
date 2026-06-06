use anyhow::Result;
use log::*;
use std::{collections::HashSet, net::SocketAddr};
use url::Url;
use yosemite::{Session, style::Stream};

/// Create virtual `SocketAddr` interfaces for given I2P trackers
pub async fn get_peers(
    id20: &[u8; 20],
    trackers: &Vec<Url>,
    announce_timeout: u64,
    trackers_proxy: Option<&Url>,
) -> Result<HashSet<SocketAddr>> {
    use btpeer::http::{announce, query::Announce};
    let mut peers = HashSet::new();
    for tracker in trackers {
        debug!("Get peers from I2P tracker `{tracker}`...");
        let mut peers_buffer = btpeer::peer::new_buffer(None);
        let a = Announce::new(tracker.as_str(), id20, 0)?;
        trace!("Sending announce `{a}`...");
        announce(
            &a,
            std::time::Duration::from_secs(announce_timeout),
            trackers_proxy.map(|url| url.as_str()),
            Some(&mut peers_buffer),
        )
        .await?;
        let t = peers_buffer.len();
        trace!("Received {t} peers total...");
        for (i, p) in peers_buffer.into_iter().enumerate() {
            trace!("Handle peer `{p}` ({i}/{t})...");
            match p.host {
                cyphernet::addr::HostName::I2p(i2p) => match new_bridge(i2p.to_string()).await {
                    Ok(peer) => {
                        let s = peer.to_string();
                        if peers.insert(peer) {
                            trace!("Inserting new I2P peer `{s}`...")
                        } else {
                            trace!("Replacing existing I2P peer `{s}`...")
                        }
                    }
                    Err(e) => warn!("Could not create now socket for peer `{i2p}`: `{e}`; skip."),
                },
                n => warn!("Unexpected I2P address family: `{n}`; skip."),
            }
        }
    }
    debug!("Collected {} unique peers total.", peers.len());
    trace!("Virtual peer bridges returned: {peers:?}");
    Ok(peers)
}

/// Create new outbound virtual stream to destination,
/// return local`SocketAddr` on success.
///
/// Destination can be:
///
/// * hostname such as `host.i2p`
/// * base32-encoded session received such as `lhbd7ojcaiofbfku7ixh47qj537g572zmhdc4oilvugzxdpdghua.b32.i2p/`
/// * base64-encoded string received from, e.g., `yosemite::Session::new`
async fn new_bridge(destination: String) -> Result<SocketAddr> {
    let loopback = std::net::SocketAddrV4::new(std::net::Ipv4Addr::new(127, 0, 0, 1), 0);

    let listener = tokio::net::TcpListener::bind(loopback).await?;
    let proxy_address = listener.local_addr()?;
    let mut session = Session::<Stream>::new(Default::default()).await?;

    tokio::spawn(async move {
        while let Ok((mut local, _)) = listener.accept().await {
            if let Ok(mut remote) = session.connect(&destination).await {
                let _ = tokio::io::copy_bidirectional(&mut local, &mut remote).await;
            }
        }
    });

    Ok(proxy_address)
}
