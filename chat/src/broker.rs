use std::collections::{HashMap, hash_map::Entry};
use async_std::stream::StreamExt;
use futures::{SinkExt, channel::mpsc, select, FutureExt};

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
    let mut peers: HashMap<String, Sender<String>> = HashMap::new();
    /* Handling Disconnections 5.
        In the broker, we create a channel to reap disconnected peers and their undelivered messages.
    */ let (
        disconnect_sender,
        mut disconnect_receiver
    ) = mpsc::unbounded();

    loop {
        let event = select! {
            event = events.next().fuse() => match event {
                Some(event) => event,
                /* Handling Disconnections 6.
                    The broker's main loop exits when the input events channel is exhausted
                    (that is, when all readers exit).
                */ None => break,
            },
            disconnect = disconnect_receiver.next().fuse() => {
                /* Handling Disconnections 7.
                    Because broker itself holds a disconnect_sender, we know that the disconnections channel
                    can't be fully drained in the main loop.
                */ let (name, _pending_messages) = disconnect.unwrap();
                assert!(peers.remove(&name).is_some());
                continue
            },
        };

        match event {
            Event::NewPeer { name, stream, shutdown } => {
                match peers.entry(name.clone()) {
                    Entry::Occupied(_) => (),
                    Entry::Vacant(entry) => {
                        let (
                            client_sender,
                            mut client_reciever
                        ) = mpsc::unbounded();
                        entry.insert(client_sender);

                        let mut disconnect_sender = disconnect_sender.clone();
                        spawn_with_loging_error(async move {
                            let res = connection_writer_loop(&mut client_reciever, stream, shutdown).await;
                            /* Handling Disconnections 8.
                                We send peer's name and pending messages to the disconnections channel in both the happy and the not-so-happy path.
                                Again, we can safely unwrap because the broker outlives writers.
                            */ disconnect_sender.send((name, client_reciever)).await.unwrap(/* with no error handling */);
                            res
                        });
                    },
                }  
            },
            Event::Message { from, to, content } => {
                for addr in to {
                    let Some(peer) = peers.get_mut(&addr) else { continue };
                    peer.send(
                        format!("from {from}: {content}\n")
                    ).await.unwrap(/* with no error handling */)
                }
            },
        }
    }

    /* Clean Shutdown 2.
        Next, the broker exits `while let Some(event) = events.next().await` loop.
    */

    /* Clean Shutdown 3.
         It's crucial that, at this stage, we drop the peers map. This drops writer's senders.
       Handling Disconnections 9.
         We drop peers map to close writers' messages channel and shut down the writers for sure.
         It is not strictly necessary in the current setup, where the broker waits for readers' shutdown anyway.
         However, if we add a server-initiated shutdown (for example, kbd:[ctrl+c] handling),
         this will be a way for the broker to shutdown the writers.
    */ drop(peers);
    /* Handling Disconnections 10.
        Finally, we close and drain the disconnections channel.
    */ drop(disconnect_sender);

    /* Clean Shutdown 4.
        Now we wait for all `disconnect_receiver`s finish their roll
    */ while let Some(
        _  // : (name, pending_messages)
    ) = disconnect_receiver.next().await {}
    // -> main

    Ok(())
}