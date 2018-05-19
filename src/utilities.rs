use futures::{Future,Poll};
use std::net::{SocketAddr, ToSocketAddrs};
use std::str;
use std::io::{self};
use std::time::Duration;
use tokio_core::reactor::{self,Handle};

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

// Here we create a timeout future, using the `Timeout::new` method,
// which will create a future that will resolve to `()` in 10 seconds.
// We then apply this timeout to the entire future all at once by
// performing a `select` between the timeout and the future itself.
pub fn timeout<T>(handle: &Handle, future:impl Future<Item=T,Error=io::Error>, msg:&'static str) 
    -> impl Future<Item=T,Error=io::Error> {
    let timeout = reactor::Timeout::new(Duration::new(10, 0), handle).unwrap();
    struct Timeout<F,TO> { f:F, m:&'static str, t:TO };
    impl<T,F,TO> Future for Timeout<F,TO>
        where F: Future<Item=T, Error=io::Error>,
              TO: Future<Item=(), Error=io::Error>
    {
        type Item = T;
        type Error = io::Error;        
        fn poll(&mut self) -> Poll<T, io::Error> {
            let Timeout {f:future, m:msg, t:timeout} = self;
            future.map(Ok).select(timeout.map(Err)).then(move |res| {
            match res {
                    // The future finished before the timeout fired, so we
                    // drop the future representing the timeout, canceling the
                    // timeout, and then return the pair of connections the
                    // handshake resolved with.
                    Ok((Ok(t), _timeout)) => Ok(t),

                    // The timeout fired before the future finished. In this
                    // case we drop the future.
                    //
                    // This automatically "cancels" any I/O associated with the
                    // handshake: reads, writes, TCP connects, etc. All of those
                    // I/O resources are owned by the future, so if we drop the
                    // future they're all released!
                    Ok((Err(()), _handshake)) => {
                        Err(other(msg))
                    }

                    // One of the futures (handshake or timeout) hit an error
                    // along the way. We're not entirely sure which at this
                    // point, but in any case that shouldn't happen, so we just
                    // keep propagating along the error.
                    Err((e, _other)) => Err(e),
                }
            }).poll()
        }
    }
    Timeout{f:future, m:msg, t:timeout}
}

//use std::mem::size_of;
//use std::intrinsics::type_name;
//pub fn print_type_info<T>(name:&str, _:&T) {
//    let s=format!("{}", unsafe { type_name::<T>() });
//    println!("{} size={},type name len={}", name, size_of::<T>(), s.len());
//}

