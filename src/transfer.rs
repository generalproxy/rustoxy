use futures::{Future,Poll,Async};
use std::rc::Rc;
use tokio_core::net::TcpStream;
use std::io::{self, Read, Write};
use std::net::Shutdown;

use buffer::{Buffer,RcBuffer};

trait Transfer: Read + Write + Future<Item=u64,Error=io::Error> {
    fn is_ready(&self) -> bool;
}

/// A future representing reading all data from one side of a proxy connection
/// and writing it to another.
///
/// This future, unlike the handshake performed above, is implemented via a
/// custom implementation of the `Future` trait rather than with combinators.
/// This is intended to show off how the combinators are not all that can be
/// done with futures, but rather more custom (or optimized) implementations can
/// be implemented with just a trait impl!
struct TcpTransfer {
    // The two I/O objects we'll be reading.
    reader: Rc<TcpStream>,
    writer: Rc<TcpStream>,

    // The shared global buffer that all connections on our server are using.
    buf: RcBuffer,

    // The number of bytes we've written so far.
    amt: u64,
    debug_string: String
}

impl Read for TcpTransfer {
    fn read(&mut self, buf:&mut [u8]) -> Result<usize, io::Error> {
        (&*self.reader).read(buf)
    }
}
impl Write for TcpTransfer {
    fn write(&mut self, buf:&[u8]) -> Result<usize, io::Error> {
        if buf.len()==0 {
            (&*self.writer).shutdown(Shutdown::Write)?;
            Ok(0)
        } else {
            (&*self.writer).write(buf)
        }
    }
    fn flush(&mut self) -> Result<(), io::Error> {
        (&*self.writer).flush()
    }
}

impl TcpTransfer {
    pub fn new(reader: Rc<TcpStream>,
           writer: Rc<TcpStream>,
           buffer: RcBuffer) -> TcpTransfer {
        let mut ds = "unknown address".to_string();
        if let (Ok(rd), Ok(wr)) = (reader.peer_addr(), writer.peer_addr()) {
            ds = format!("{:?} -> {:?}", rd, wr);
        }

        TcpTransfer {
            reader: reader,
            writer: writer,
            buf: buffer,
            amt: 0,
            debug_string: ds
        }
    }        
}

// Here we implement the `Future` trait for `Transfer` directly. This does not
// use any combinators, and shows how you might implement it in custom
// situations if needed.
impl Future for TcpTransfer {
    // Our future resolves to the number of bytes transferred, or an I/O error
    // that happens during the connection, if any.
    type Item = u64;
    type Error = io::Error;

    /// Attempts to drive this future to completion, checking if it's ready to
    /// be completed.
    ///
    /// This method is the core foundation of completing a future over time. It
    /// is intended to never block and return "quickly" to ensure that it
    /// doesn't block the event loop.
    ///
    /// Completion for our `Transfer` future is defined when one side hits EOF
    /// and we've written all remaining data to the other side of the
    /// connection. The behavior of `Future::poll` is in general not specified
    /// after a future resolves (e.g. in this case returns an error or how many
    /// bytes were transferred), so we don't need to maintain state beyond that
    /// point.
    fn poll(&mut self) -> Poll<u64, io::Error> {
        //let mut buffer = self.buf.borrow_mut();

        // Here we loop over the two TCP halves, reading all data from one
        // connection and writing it to another. The crucial performance aspect
        // of this server, however, is that we wait until both the read half and
        // the write half are ready on the connection, allowing the buffer to
        // only be temporarily used in a small window for all connections.
        loop {
           if !self.is_ready() {
                return Ok(Async::NotReady)
            }

            // TODO: This exact logic for reading/writing amounts may need an
            //       update
            //
            // Right now the `buffer` is actually pretty big, 64k, and it could
            // be the case that one end of the connection can far outpace
            // another. For example we may be able to always read 64k from the
            // read half but only be able to write 5k to the client. This is a
            // pretty bad situation because we've got data in a buffer that's
            // intended to be ephemeral!
            //
            // Ideally here we'd actually adapt the rate of reads to match the
            // rate of writes. That is, we'd prefer to have some form of
            // adaptive algorithm which keeps track of how many bytes are
            // written and match the read rate to the write rate. It's possible
            // for connections to have an even smaller (and optional) buffer on
            // the side representing the "too much data they read" if that
            // happens, and then the next call to `read` could compensate by not
            // reading so much again.
            //
            // In any case, though, this is easily implementable in terms of
            // adding fields to `Transfer` and is complicated enough to
            // otherwise detract from the example in question here. As a result,
            // we simply read into the global buffer and then assert that we
            // write out exactly the same amount.
            //
            // This means that we may trip the assert below, but it should be
            // relatively easily fixable with the strategy above!

            //let n = try_nb!((&*self.reader).read(&mut buffer));
            let n = try_nb!(self.buf.clone().read(self));
            info!("received {} bytes ({})", n, self.debug_string);
            self.amt += n as u64;

            // Unlike above, we don't handle `WouldBlock` specially, because
            // that would play into the logic mentioned above (tracking read
            // rates and write rates), so we just ferry along that error for
            // now.
            //let m = try!((&*self.writer).write(&buffer[..n]));
            let m = self.buf.clone().write(self, n)?;
            assert_eq!(n, m);
        }
    }
}

impl Transfer for TcpTransfer {
    fn is_ready(&self) -> bool {
        let read_ready = self.reader.poll_read().is_ready();
        let write_ready = self.writer.poll_write().is_ready();
        return read_ready && write_ready;
    }
}

pub fn transfer(ep1: TcpStream, ep2: TcpStream, buffer: RcBuffer) 
    -> impl Future<Item=(u64,u64), Error=io::Error> {
    let ep1 = Rc::new(ep1);
    let ep2 = Rc::new(ep2);

    let half1 = TcpTransfer::new(ep1.clone(), ep2.clone(), buffer.clone());
    let half2 = TcpTransfer::new(ep2, ep1, buffer);
    half1.join(half2)
}