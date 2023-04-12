use std::{env, net::SocketAddr};

use async_tungstenite::{gio::accept_async, tungstenite::Result};
use futures::prelude::*;
use gio::{
    prelude::*, InetSocketAddress, SocketConnection, SocketProtocol, SocketService, SocketType,
};

async fn accept_connection(stream: SocketConnection) -> Result<()> {
    let addr = stream
        .socket()
        .remote_address()
        .expect("SocketConnection should have a remote address");

    println!("Peer address: {}", addr);
    let mut ws_stream = accept_async(stream)
        .await
        .expect("Error during the websocket handshake occurred");

    while let Some(msg) = ws_stream.next().await {
        let msg = msg?;
        if msg.is_text() || msg.is_binary() {
            ws_stream.send(msg).await?;
        }
    }

    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _ = env_logger::try_init();
    let addr = env::args()
        .nth(1)
        .unwrap_or_else(|| "127.0.0.1:8080".to_string());

    let sockaddr: SocketAddr = addr.parse()?;
    let inetaddr: InetSocketAddress = sockaddr.into();

    let service = SocketService::new();
    service.add_address(
        &inetaddr,
        SocketType::Stream,
        SocketProtocol::Tcp,
        glib::Object::NONE,
    )?;
    println!("Listening on: {}", inetaddr);

    service.connect_incoming(|_service, connection, _| {
        let stream = connection.clone();
        glib::MainContext::default().spawn_local(async move {
            accept_connection(stream).await.unwrap();
        });
        false
    });

    let main_loop = glib::MainLoop::new(None, false);
    main_loop.run();

    Ok(())
}
