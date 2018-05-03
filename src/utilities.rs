use futures::{Future,Poll};
use std::net::{SocketAddr, ToSocketAddrs};
use std::str;
use std::io::{self};

// Mix two different Future into one,
// to be used for impl Trait feature.
pub enum EitherFuture<L,R,T,E>
    where L:Future<Item=T,Error=E>,
          R:Future<Item=T,Error=E>
{
    Left(L),
    Right(R)
}

use self::EitherFuture::{Left,Right};

impl<L,R,T,E> Future for EitherFuture<L,R,T,E>
    where L:Future<Item=T,Error=E>,
          R:Future<Item=T,Error=E>
{
    type Item=T;
    type Error=E;
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        match self {
            Left(f) => f.poll(),
            Right(f) => f.poll()
        }
    }
}

// Extracts the name and port from addr_buf and returns them, converting
// the name to the form that the trust-dns client can use. If the original
// name can be parsed as an IP address, makes a SocketAddr from that
// address and the port and returns it; we skip DNS resolution in that
// case.
pub fn name_port(addr_buf: &[u8]) -> io::Result<SocketAddr> {
    // The last two bytes of the buffer are the port, and the other parts of it
    // are the hostname.
    let hostname = &addr_buf[..addr_buf.len() - 2];
    let hostname = try!(str::from_utf8(hostname).map_err(|_e| {
        other("hostname buffer provided was not valid utf-8")
    }));
    let pos = addr_buf.len() - 2;
    let port = ((addr_buf[pos] as u16) << 8) | (addr_buf[pos + 1] as u16);

    if let Ok(ip) = hostname.parse() {
        return Ok(SocketAddr::new(ip, port))
    }

    format!("{}:{}",hostname,port).to_socket_addrs()?.next()
    .map_or(Err(other("host name didn't resolve to valid IP address")), |a| {
        info!("target: {}:{} = {:?}", hostname, port, a);
        Ok(a)
    })
}

pub fn other(desc: &str) -> io::Error {
    io::Error::new(io::ErrorKind::Other, desc)
}