#![feature(box_syntax)]

//! An example [SOCKSv5] proxy server on top of futures
//!
//! [SOCKSv5]: https://www.ietf.org/rfc/rfc1928.txt
//!
//! This program is intended to showcase many aspects of the futures crate and
//! I/O integration, explaining how many of the features can interact with one
//! another and also provide a concrete example to see how easily pieces can
//! interoperate with one another.
//!
//! A SOCKS proxy is a relatively easy protocol to work with. Each TCP
//! connection made to a server does a quick handshake to determine where data
//! is going to be proxied to, another TCP socket is opened up to this
//! destination, and then bytes are shuffled back and forth between the two
//! sockets until EOF is reached.
//!
//! This server implementation is relatively straightforward, but
//! architecturally has a few interesting pieces:
//!
//! * The entire server only has one buffer to read/write data from. This global
//!   buffer is shared by all connections and each proxy pair simply reads
//!   through it. This is achieved by waiting for both ends of the proxy to be
//!   ready, and then the transfer is done.
//!
//! * Initiating a SOCKS proxy connection may involve a DNS lookup, which
//!   is done with the TRust-DNS futures-based resolver. This demonstrates the
//!   ease of integrating a third-party futures-based library into our futures
//!   chain.
//!
//! * The entire SOCKS handshake is implemented using the various combinators in
//!   the `futures` crate as well as the `tokio_core::io` module. The actual
//!   proxying of data, however, is implemented through a manual implementation
//!   of `Future`. This shows how it's easy to transition back and forth between
//!   the two, choosing whichever is the most appropriate for the situation at
//!   hand.
//!
//! You can try out this server with `cargo test` or just `cargo run` and
//! throwing connections at it yourself, and there should be plenty of comments
//! below to help walk you through the implementation as well!

#[macro_use]
extern crate log;
extern crate env_logger;
extern crate futures;
#[macro_use]
extern crate tokio_core;
extern crate tokio_io;

mod transfer;
mod client;
mod client_channel;
mod either_future;
mod buffer;

use std::env;
use std::net::SocketAddr;

use futures::future;
use futures::{Future, Stream};
use tokio_core::reactor::Core;

use client_channel::{ClientChannel, listen_tcp};

use buffer::Buffer;

fn main() {
    env::set_var("RUST_LOG", "info");
    drop(env_logger::init());

    // Take the first command line argument as an address to listen on, or fall
    // back to just some localhost default.
    let addr = env::args().nth(1).unwrap_or("127.0.0.1:8083".to_string());
    let addr = addr.parse::<SocketAddr>().unwrap();

    // Initialize the various data structures we're going to use in our server.
    // Here we create the event loop, the global buffer that all threads will
    // read/write into, and the bound TCP listener itself.
    let mut lp = Core::new().unwrap();
    let buffer = Buffer::new();
    let handle = lp.handle();

    // Construct a future representing our server. This future processes all
    // incoming connections and spawns a new task for each client which will do
    // the proxy work.
    //
    // This essentially means that for all incoming connections, those received
    // from `listener`, we'll create an instance of `Client` and convert it to a
    // future representing the completion of handling that client. This future
    // itself is then *spawned* onto the event loop to ensure that it can
    // progress concurrently with all other connections.
    let channel = listen_tcp(&addr, &handle).unwrap();
    //let listener = TcpListener::bind(&addr, &handle).unwrap();
    info!("Listening for socks5 proxy connections on {}", addr);
    //let clients = listener.incoming().map(move |(socket, addr)| {
    //    info!("connected: {:?}", addr);
    //    Client::new(&buffer, &handle, addr)
    //});
    let handle = lp.handle();
    let server = channel.clients(buffer, &handle).for_each(|client| {
            let addr = client.get_addr();
            handle.spawn(client.serve().then(move |res| {
                match res {
                    Ok((a, b)) => {
                        info!("proxied {}/{} bytes for {}", a, b, addr)
                    }
                    Err(e) => error!("error for {}: {}", addr, e),
                }
                future::ok(())
            }));
        Ok(())
    });

    lp.run(server).unwrap();
}