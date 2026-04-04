use async_tungstenite::{
    accept_async,
    tungstenite::{Error, Result},
};
use futures::prelude::*;
use log::*;
use smol::net::{SocketAddr, TcpListener, TcpStream};

async fn accept_connection(peer: SocketAddr, stream: TcpStream) {
    if let Err(e) = handle_connection(peer, stream).await {
        match e {
            Error::ConnectionClosed | Error::Protocol(_) | Error::Utf8(_) => (),
            err => error!("Error processing connection: {}", err),
        }
    }
}

async fn handle_connection(peer: SocketAddr, stream: TcpStream) -> Result<()> {
    let mut ws_stream = accept_async(stream).await.expect("Failed to accept");

    info!("New WebSocket connection: {}", peer);

    while let Some(msg) = ws_stream.next().await {
        let msg = msg?;
        if msg.is_text() || msg.is_binary() {
            // here we explicitly using futures 0.3's Sink implementation for send message
            // for WebSocketStream::send, see autobahn-client example
            futures::SinkExt::send(&mut ws_stream, msg).await?;
        }
    }

    Ok(())
}

async fn run() {
    env_logger::init();

    let addr = "127.0.0.1:9002";
    let listener = TcpListener::bind(&addr).await.expect("Can't listen");
    info!("Listening on: {}", addr);

    while let Ok((stream, _)) = listener.accept().await {
        let peer = stream
            .peer_addr()
            .expect("connected streams should have a peer address");
        info!("Peer address: {}", peer);

        smol::spawn(accept_connection(peer, stream)).detach();
    }
}

fn main() {
    smol::block_on(run());
}
