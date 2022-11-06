use std::sync::Arc;
use async_std::{
    net::TcpStream,
    stream::StreamExt,
    io::{
        BufReader,
        prelude::BufReadExt
    },
};
use futures::{SinkExt, channel::mpsc};

use crate::utils::types::{
    Result,
    Sender,
    Event,
    Void,
};


pub(super) async fn connection_loop(mut broker: Sender<Event>, stream: TcpStream) -> Result<()> {
    let stream = Arc::new(stream);
    let reader = BufReader::new(&*stream);
    let mut lines = reader.lines();

    let name = match lines.next().await {
        None => Err("peer disconnected immediately")?,
        Some(line) => line?,
    };

    /* Handling Disconnections 2.
        In the reader, we create a _shutdown_sender whose only purpose is to get dropped.
    */ let (
        _shutdown_sender,
        shutdown_reciever
    ) = mpsc::unbounded::<Void>();  // -> sender

    broker.send(Event::NewPeer {
        name:     name.clone(),
        stream:   Arc::clone(&stream),
        shutdown: shutdown_reciever,
    }).await.unwrap(/* with no error handling */);

    while let Some(line) = lines.next().await {
        let line = line?;
        let Some((dest, msg)) = line.split_once(':') else { continue };
        broker.send(Event::Message {
            from:    name.clone(),
            to:      dest.split(',').map(|name| name.trim().into()).collect(),
            content: msg.into(),
        }).await.unwrap(/* with no error handling */);
    }

    Ok(())
}
