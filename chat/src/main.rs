mod utils;
mod reciever;
mod sender;
mod broker;


use async_std::{
    task,
    stream::StreamExt,
    net::{
        TcpListener, TcpStream,
    }, io::{BufReader, prelude::BufReadExt, stdin, WriteExt},
};
use futures::{channel::mpsc, select, FutureExt};

use utils::{
    types::Result,
    funcs::spawn_with_loging_error,
    consts::{
        TCP_ADDRESS,
        LISTENING_PORT,
    },
};
use reciever::connection_loop;
use broker::broker_loop;


async fn run_server() -> Result<()> {
    let listener = TcpListener::bind(
        format!("{TCP_ADDRESS}:{LISTENING_PORT}")
    ).await?;
    let mut incoming = listener.incoming();

    let (broker_sender, broker_reciever) = mpsc::unbounded();
    let broker_handle = task::spawn(
        broker_loop(broker_reciever)
    );

    while let Some(stream) = incoming.next().await {
        let stream = stream?;
        println!("Accepting from: {}", stream.peer_addr()?);

        spawn_with_loging_error(
            connection_loop(broker_sender.clone(), stream)
        );
    }


    // for Clean Shutdown
    /*
        One of the problems of the current implementation is that it doesn't handle graceful shutdown.
        If we break from the accept loop for some reason, all in-flight tasks are just dropped on the floor.
        A more correct shutdown sequence would be:

            Stop accepting new clients  ->  Deliver all pending messages  ->  Exit the process
    */
    /*
        A clean shutdown in a channel based architecture is easy, although it can appear a magic trick at first.
        In Rust, receiver side of a channel is closed as soon as all senders are dropped. That is, as soon as
        producers exit and drop their senders, the rest of the system shuts down naturally. In async_std this
        translates to two rules:

            - Make sure that channels form an acyclic graph.
            - Take care to wait, in the correct order, until intermediate layers of the system process pending messages.

        In this chat program, we already have an unidirectional flow of messages: reader -> broker -> writer.
        However, we never wait for broker and writers, which might cause some messages to get dropped.
        Let's add waiting to the server:
    */
    /* Clean Shutdown 1.
        First, we drop the main broker's sender. That way when the readers are done,
        there's no sender for the broker's channel, and the chanel closes.
    */  drop(broker_sender);  // -> broker
    /* Clean Shutdown 5.
        Finally, we join the broker, which also guarantees that all the writes have terminated.
    */  broker_handle.await?;


    // for Handling Disconnecions
    /*
        Currently, we only ever add new peers to the map. This is clearly wrong: if a peer closes connection to the chat,
        we should not try to send any more messages to it.
    */
    /*
        One subtlety with handling disconnection is that we can detect it either in the reader's task, or in the writer's task.
        The most obvious solution here is to just remove the peer from the peers map in both cases, but this would be wrong.
        If both read and write fail, we'll remove the peer twice, but it can be the case that the peer reconnected between the two failures!
        To fix this, we will only remove the peer when the write side finishes. If the read side finishes we will notify the write side that
        it should stop as well. That is, we need to add an ability to signal shutdown for the writer task.
    */
    /*
        One way to approach this is a shutdown: Receiver<()> channel. There's a more minimal solution however, which makes clever use of RAII.
        Closing a channel is a synchronization event, so we don't need to send a shutdown message, we can just drop the sender. This way,
        we statically guarantee that we issue shutdown exactly once, even if we early return via ? or panic.
    */
    // -> util::types


    Ok(())
}

async fn run_client() -> Result<()> {
    let stream = TcpStream::connect(
        format!("{TCP_ADDRESS}:{LISTENING_PORT}")
    ).await?;
    // Here we split TcpStream into read and write halves: there's `impl AsyncRead for &'_ TcpStream`,
    // just like the one in std.
    let (reader, mut writer) = (&stream, &stream);

    let mut lines_from_server = BufReader::new(reader).lines().fuse();
    let mut lines_from_stdin = BufReader::new(stdin()).lines().fuse();

    loop {
        select! {
            line = lines_from_server.next().fuse() => match line {
                None => break,
                Some(line) => println!("{}", line?),
            },
            line = lines_from_stdin.next().fuse() => match line {
                None => break,
                Some(line) => writer.write_all((line? + "\n").as_bytes()).await?
            },
        }
    }

    Ok(())
}

fn main() -> Result<()> {
    task::block_on(async {
        run_server().await?;
        run_client().await?;
        Ok(())
    })
}
