use tokio_core::net::TcpStream;
use tokio_io::{io::copy,AsyncRead,AsyncWrite};
use std::rc::Rc;
use std::io::{self,Read,Write};
use std::net::Shutdown;
use futures::Poll;
use futures::{Future,future::{result,ok}};

pub trait Endpoint {
    type ReadHalf: AsyncRead;
    type WriteHalf: AsyncWrite;
    fn split(self) -> (Self::ReadHalf, Self::WriteHalf);
}

pub fn new_tcpendpoint(s: TcpStream) -> impl Endpoint {
    let mut ds = "unknown address".to_string();
    if let Ok(add) = s.peer_addr() {
        ds = format!("{:?}", add);
    }
    #[derive(Clone)]
    struct TcpEndpoint {
        stream: Rc<TcpStream>,
        debug_string: String
    }
    impl TcpEndpoint {
        fn as_async<'a>(&'a self) -> impl AsyncRead + AsyncWrite + 'a {
            &*self.stream
        }
    }
    impl Read for TcpEndpoint {
        fn read(&mut self, buf: &mut [u8]) -> Result<usize, io::Error> {        
            self.as_async().read(buf)
        }
    }
    impl AsyncRead for TcpEndpoint {
        fn poll_read(&mut self, buf: &mut [u8]) -> Poll<usize, io::Error> {
            info!("received {} bytes ({})", buf.len(), self.debug_string);
            self.as_async().poll_read(buf)
        }
    }
    impl Write for TcpEndpoint {
        fn write(&mut self, buf: &[u8]) -> Result<usize, io::Error> {
            if buf.len()==0 {
                self.as_async().shutdown()?;
                Ok(0)
            } else {
                self.as_async().write(buf)
            }
        }
        fn flush(&mut self) -> Result<(), io::Error> {
            self.as_async().flush()
        }
    }
    impl AsyncWrite for TcpEndpoint {
        fn shutdown(&mut self) -> Poll<(), io::Error> {
            result(self.as_async().flush().and_then(|()|
                (&*self.stream).shutdown(Shutdown::Write))).poll()
        }
    }
    impl Endpoint for TcpEndpoint {
        type ReadHalf = Self;
        type WriteHalf = Self;
        fn split(self) -> (Self::ReadHalf, Self::WriteHalf) {
            (self.clone(), self)
        }
    }
    TcpEndpoint { stream: Rc::new(s),  debug_string: ds }
}

pub fn transfer(ep1: impl Endpoint, ep2: impl Endpoint)
    -> impl Future<Item=(u64,u64), Error=io::Error>
{
    let (ep1r, ep1w) = ep1.split();
    let (ep2r, ep2w) = ep2.split();
    copy(ep1r, ep2w).join(copy(ep2r, ep1w))
        .and_then(|(v1,v2)| ok((v1.0, v2.0)))
}

