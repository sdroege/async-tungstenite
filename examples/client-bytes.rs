//! A simple example of hooking up stdin/stdout to a WebSocket stream using ByteStream.
//!
//! This example will connect to a server specified in the argument list and
//! then forward all data read on stdin to the server, printing out all data
//! received on stdout.
//!
//! Note that this is not currently optimized for performance, especially around
//! buffer management. Rather it's intended to show an example of working with a
//! client.
//!
//! You can use this example together with the `server` example.

use std::env;

use futures::StreamExt;

use async_std::io;
use async_std::prelude::*;
use async_std::task;
use async_tungstenite::async_std::connect_async;
use async_tungstenite::ByteWriter;

async fn run() {
    let connect_addr = env::args()
        .nth(1)
        .unwrap_or_else(|| panic!("this program requires at least one argument"));

    let (ws_stream, _) = connect_async(&connect_addr)
        .await
        .expect("Failed to connect");
    println!("WebSocket handshake has been successfully completed");

    let (write, read) = ws_stream.split();
    let byte_writer = ByteWriter::new(write);
    let stdin_to_ws = task::spawn(async {
        io::copy(io::stdin(), byte_writer).await.unwrap();
    });
    let ws_to_stdout = task::spawn(read.for_each(|message| async {
        let data = message.unwrap().into_data();
        async_std::io::stdout().write_all(&data).await.unwrap();
    }));
    stdin_to_ws.await;
    ws_to_stdout.await;
}

fn main() {
    task::block_on(run())
}
