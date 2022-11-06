use std::sync::Arc;
use async_std::{
    io::WriteExt,
    net::TcpStream,
    stream::StreamExt,
};
use futures::{select, FutureExt};
use crate::utils::types::{
    Reciever,
    Result,
    Void,
};


pub(super) async fn connection_writer_loop(
    messages: &mut Reciever<String>,
    stream:   Arc<TcpStream>,
    /* Handling Disconnections 3.
        We add shutdown channel as an argument.
    */ shutdown: Reciever<Void>,
) -> Result<()> {
    let mut stream = &*stream;

    let mut messages = messages.fuse();
    let mut shutdown = shutdown.fuse();

    /* Handling Disconnections 4.
        In the connection_writer_loop, we now need to choose between shutdown and message channels.
        We use the select macro for this purpose:
    */ loop {  // Because of select, we can't use a while let loop, so we desugar it further into a loop.
        select! {
            // fuse() is used to turn any `Stream` into a `FusedStream`. This is used for fusing a stream
            // such that poll_next will never again be called once it has finished.
            msg = messages.next().fuse() => match msg {
                Some(msg) => stream.write_all(msg.as_bytes()).await?,
                None => break,
            },
            void = shutdown.next().fuse() => match void {
                // In the shutdown case we use `match void {}` as a statically-checked unreachable!().
                Some(void) => match void {},
                None => break,
            },
        }
    }

    /*
        Another problem is that between the moments of
            - we detect disconnection in connection_writer_loop
            - when we actually remove the peer from the peers map
        there new messages might be pushed into the peer's channel. To not lose these messages completely,
        we'll return the messages channel back to the broker. This also allows us to establish a useful invariant that
        the message channel strictly outlives the peer in the peers map, and makes the broker itself infallible.
    */  // -> broker

    Ok(())
}