use std::collections::{HashMap, hash_map::Entry};
use async_std::stream::StreamExt;
use futures::{SinkExt, channel::mpsc};

use crate::{
    utils::{
        types::{
            Reciever,
            Result,
            Event, Sender,
        },
        funcs::spawn_with_loging_error
    },
    sender::connection_writer_loop
};


pub(super) async fn broker_loop(mut events: Reciever<Event>) -> Result<()> {
    let mut writers = Vec::new();
    let mut peers: HashMap<String, Sender<String>> = HashMap::new();

    while let Some(event) = events.next().await {
        match event {
            Event::NewPeer { name, stream } => {
                match peers.entry(name) {
                    Entry::Occupied(_) => (),
                    Entry::Vacant(entry) => {
                        let (client_sender, client_reciever) = mpsc::unbounded();
                        entry.insert(client_sender);
                        let handle = spawn_with_loging_error(
                            connection_writer_loop(client_reciever, stream)
                        );
                        writers.push(handle);
                    },
                }  
            },
            Event::Message { from, to, content } => {
                for addr in to {
                    let Some(peer) = peers.get_mut(&addr) else { continue };
                    peer.send(
                        format!("from {from}: {content}\n")
                    ).await?
                }
            },
        }
    }

    /* Clean Shutdown 2.
        Next, the broker exits `while let Some(event) = events.next().await` loop.
    */
    /* Clean Shutdown 3.
        It's crucial that, at this stage, we drop the peers map. This drops writer's senders.
    */  drop(peers);
    /* Clean Shutdown 4.
        Now we can join all of the writers.
    */  for writer in writers {
        writer.await;
    }   // -> main

    Ok(())
}