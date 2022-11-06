use std::sync::Arc;

use async_std::net::TcpStream;
use futures::channel::mpsc;


pub(crate) type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

pub(crate) type Sender<T> = mpsc::UnboundedSender<T>;
pub(crate) type Reciever<T> = mpsc::UnboundedReceiver<T>;

#[derive(Debug)]
pub(crate) enum Void {}

#[derive(Debug)]
pub(crate) enum Event {
    NewPeer {
        name:     String,
        stream:   Arc<TcpStream>,
        /* Handling Disconnections 1.
            To enforce that no messages are sent along the shutdown channel, we use an uninhabited type.
            We pass the shutdown channel to the writer task.
        */  shutdown: Reciever<Void>,  // -> reciever
    },
    Message {
        from:    String,
        to:      Vec<String>,
        content: String,
    },
}