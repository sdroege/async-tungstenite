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

use async_tungstenite::smol::connect_async;
use async_tungstenite::{ByteReader, ByteWriter};

use smol::io::AsyncWriteExt;
use smol::Unblock;

async fn run() {
    let connect_addr = env::args()
        .nth(1)
        .unwrap_or_else(|| panic!("this program requires at least one argument"));

    let (ws_stream, _) = connect_async(&connect_addr)
        .await
        .expect("Failed to connect");
    println!("WebSocket handshake has been successfully completed");

    let (write, read) = ws_stream.split();

    let stdin_to_ws: smol::Task<std::io::Result<()>> = smol::spawn(async {
        let mut stdin = Unblock::new(std::io::stdin());
        let mut byte_writer = ByteWriter::new(write);
        smol::io::copy(&mut stdin, &mut byte_writer).await?;
        byte_writer.flush().await
    });

    let ws_to_stdout: smol::Task<std::io::Result<()>> = smol::spawn(async {
        let mut byte_reader = ByteReader::new(read);
        let mut stdout = Unblock::new(std::io::stdout());
        smol::io::copy(&mut byte_reader, &mut stdout).await?;
        stdout.flush().await
    });

    stdin_to_ws.await.unwrap();
    ws_to_stdout.await.unwrap();
}

fn main() {
    smol::block_on(run())
}
