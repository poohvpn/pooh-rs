use async_std::net::{SocketAddr, SocketAddrV4, SocketAddrV6, TcpStream, ToSocketAddrs, UdpSocket};
use async_trait::async_trait;
use socket2::{Domain, Protocol, Socket, Type};
use std::io;
use std::option::Option::Some;
use std::prelude::v1::Result::Ok;

pub const STREAM_BUF_SIZE: usize = 32 * 1024;

pub const DATAGRAM_BUF_SIZE: usize = 64 * 1024;

pub const FRAME_LENGTH_SIZE: usize = 2;

const LOCAL_LISTEN_ADDR: &str = "127.19.89.64:0";

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum BindType {
    IPv4Tcp,
    IPv4Udp,
    IPv4Icmp,
    IPv6Tcp,
    IPv6Udp,
    IPv6Icmp,
}

pub fn bind(r#type: BindType, addr: &str) -> io::Result<Socket> {
    let sock = match r#type {
        BindType::IPv4Tcp => Socket::new(Domain::ipv4(), Type::stream(), None)?,
        BindType::IPv4Udp => Socket::new(Domain::ipv4(), Type::dgram(), None)?,
        BindType::IPv4Icmp => Socket::new(Domain::ipv4(), Type::raw(), Some(Protocol::icmpv4()))?,
        BindType::IPv6Tcp => Socket::new(Domain::ipv6(), Type::stream(), None)?,
        BindType::IPv6Udp => Socket::new(Domain::ipv6(), Type::dgram(), None)?,
        BindType::IPv6Icmp => Socket::new(Domain::ipv6(), Type::raw(), Some(Protocol::icmpv6()))?,
    };
    match r#type {
        BindType::IPv6Tcp | BindType::IPv6Udp => {
            sock.set_only_v6(true)?;
            sock.bind(&addr.parse::<std::net::SocketAddrV6>().unwrap().into())?;
        }
        _ => {
            sock.bind(&addr.parse::<std::net::SocketAddrV4>().unwrap().into())?;
        }
    };
    match r#type {
        BindType::IPv4Tcp | BindType::IPv6Tcp => {
            sock.listen(16)?;
        }
        _ => {}
    };
    Ok(sock)
}

pub fn strip_ipv4_header(b: &[u8]) -> &[u8] {
    if b.len() < 20 {
        return b;
    }
    if b[0] >> 4 != 4 {
        return b;
    }
    let l = ((b[0] & 0x0f) as usize) << 2;
    if 20 > l || l > b.len() {
        return b;
    }
    return &b[l..];
}

#[async_trait(?Send)]
pub trait SocketAddrExt: ToSocketAddrs {
    async fn dial_tcp(&self) -> io::Result<TcpStream> {
        TcpStream::connect(self).await
    }

    async fn dial_udp(&self) -> io::Result<UdpSocket> {
        let sock = UdpSocket::bind("0.0.0.0:0").await?;
        sock.connect(self).await?;
        Ok(sock)
    }

    async fn dial_icmpv4(&self) -> io::Result<UdpSocket> {
        let sock =
            Socket::new(Domain::ipv4(), Type::raw(), Some(Protocol::icmpv4()))?.into_udp_socket();
        let sock = UdpSocket::from(sock);
        sock.connect(self).await?;
        Ok(sock)
    }
    async fn dial_icmpv6(&self) -> io::Result<UdpSocket> {
        let sock =
            Socket::new(Domain::ipv6(), Type::raw(), Some(Protocol::icmpv6()))?.into_udp_socket();
        let sock = UdpSocket::from(sock);
        sock.connect(self).await?;
        Ok(sock)
    }
}

#[async_trait(?Send)]
impl<T: ToSocketAddrs> SocketAddrExt for T {}

fn any_success<T>(res1: io::Result<T>, res2: io::Result<T>) -> io::Result<(Option<T>, Option<T>)> {
    match res1 {
        Ok(sock1) => Ok(match res2 {
            Ok(sock2) => (Some(sock1), Some(sock2)),
            _ => (Some(sock1), None),
        }),
        Err(err1) => match res2 {
            Ok(err2) => Ok((None, Some(err2))),
            _ => Err(err1),
        },
    }
}

#[derive(Debug, Copy, Clone)]
pub enum DualAddr {
    V4(SocketAddrV4),
    V6(SocketAddrV6),
    Both(SocketAddrV4, SocketAddrV6),
}

impl DualAddr {
    pub async fn dial_tcp(self) -> io::Result<(Option<TcpStream>, Option<TcpStream>)> {
        match self {
            DualAddr::V4(addr) => Ok((Some(addr.dial_tcp().await?), None)),
            DualAddr::V6(addr) => Ok((None, Some(addr.dial_tcp().await?))),
            DualAddr::Both(addr_v4, addr_v6) => {
                any_success(addr_v4.dial_tcp().await, addr_v6.dial_tcp().await)
            }
        }
    }

    pub async fn dial_udp(&self) -> io::Result<(Option<UdpSocket>, Option<UdpSocket>)> {
        match self {
            DualAddr::V4(addr) => Ok((Some(addr.dial_udp().await?), None)),
            DualAddr::V6(addr) => Ok((None, Some(addr.dial_udp().await?))),
            DualAddr::Both(addr_v4, addr_v6) => {
                any_success(addr_v4.dial_udp().await, addr_v6.dial_udp().await)
            }
        }
    }

    pub async fn dial_icmp(&self) -> io::Result<(Option<UdpSocket>, Option<UdpSocket>)> {
        match self {
            DualAddr::V4(addr) => Ok((Some(addr.dial_icmpv4().await?), None)),
            DualAddr::V6(addr) => Ok((None, Some(addr.dial_icmpv6().await?))),
            DualAddr::Both(addr_v4, addr_v6) => {
                any_success(addr_v4.dial_icmpv4().await, addr_v6.dial_icmpv6().await)
            }
        }
    }
}

pub fn new_udp_pair() -> io::Result<(UdpSocket, UdpSocket)> {
    let zero = bind(BindType::IPv4Udp, LOCAL_LISTEN_ADDR).map(|sock| sock.into_udp_socket())?;
    let one = bind(BindType::IPv4Udp, LOCAL_LISTEN_ADDR).map(|sock| sock.into_udp_socket())?;
    zero.connect(&one.local_addr()?)?;
    one.connect(&zero.local_addr()?)?;
    Ok((zero.into(), one.into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_udp_pair() {
        let (a, b) = new_udp_pair().unwrap();
        println!("{:?}", a.local_addr().unwrap());
        println!("{:?}", b.local_addr().unwrap());
        assert_eq!(a.local_addr().unwrap(), b.peer_addr().unwrap());
        assert_eq!(a.peer_addr().unwrap(), b.local_addr().unwrap());
    }

    #[test]
    fn test_dial() {
        let server = bind(BindType::IPv4Udp, "0.0.0.0:8964").unwrap();
        let client = SocketAddr::from(([127, 0, 0, 1], 8964)).dial_udp().unwrap();
        let data = &[1, 2, 3, 4u8];
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(client.send(data)).unwrap();
        let buf = &mut [0u8; 1 << 16];
        let n = server.recv(buf).unwrap();
        assert_eq!(n, data.len());
        assert_eq!(data, &buf[..n]);
    }
}
