use tokio_core::reactor::Handle;
use futures::Async::{Ready,NotReady};
use futures::Async;
use client::Client;
use futures::Stream;
use tokio_core::net::Incoming;
use std::net::SocketAddr;
use tokio_core::net::TcpListener;
use std::io;

use buffer::RcBuffer;

pub trait ClientChannel {
    type OutputStream: Stream<Item=Client, Error=io::Error>;
    fn clients(self, buffer: RcBuffer, handle:&Handle) -> Self::OutputStream;
}

pub fn listen_tcp(addr: &SocketAddr, handle:&Handle) -> io::Result<impl ClientChannel> {
    TcpListenerChannel::new(addr, handle)
}

struct TcpClientStream {
    s: Incoming,
    h: Handle,
    b: RcBuffer
}

struct TcpListenerChannel {
    listener: TcpListener
}

impl TcpListenerChannel {
    fn new(addr: &SocketAddr, handle:&Handle) -> io::Result<TcpListenerChannel> {
        TcpListener::bind(addr, handle).map(|l| TcpListenerChannel { listener: l })
    }
}

impl Stream for TcpClientStream {
    type Item = Client;
    type Error = io::Error;
    fn poll(&mut self) -> io::Result<Async<Option<Client>>> {
        match self.s.poll() {
            Ok(Ready(Some((c,a)))) => Ok(Ready(Some(Client::new(c,&self.b,&self.h,a)))),
            Ok(Ready(None)) => Ok(Ready(None)),
            Ok(NotReady) => Ok(NotReady),
            Err(e) => Err(e)
        }
    }
}

impl ClientChannel for TcpListenerChannel {
    type OutputStream = TcpClientStream;
    fn clients(self, buffer: RcBuffer, handle:&Handle) -> TcpClientStream {
        TcpClientStream { s: self.listener.incoming(), h:handle.clone(), b:buffer }
    }
}