use anyhow::Result;
use std::{
    net::{Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6, UdpSocket},
    time::Duration,
};

pub fn server_for_socket(socket: &SocketAddr, timeout: u64) -> Result<UdpSocket> {
    let duration = Duration::from_secs(timeout);

    let server = if socket.is_ipv6() {
        UdpSocket::bind(SocketAddr::V6(SocketAddrV6::new(
            Ipv6Addr::UNSPECIFIED,
            0,
            0,
            0,
        )))?
    } else {
        UdpSocket::bind(SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 0)))?
    };

    server.set_read_timeout(Some(duration))?;
    server.set_write_timeout(Some(duration))?;

    Ok(server)
}
