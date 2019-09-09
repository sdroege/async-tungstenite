use futures::StreamExt;
use log::*;
use std::net::ToSocketAddrs;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::tcp::{TcpListener, TcpStream};
use tokio_tungstenite::{accept_async, client_async, WebSocketStream};
use tungstenite::Message;

async fn run_connection<S>(
    connection: WebSocketStream<S>,
    msg_tx: futures::channel::oneshot::Sender<Vec<Message>>,
) where
    S: AsyncRead + AsyncWrite + Unpin,
{
    info!("Running connection");
    let mut connection = connection;
    let mut messages = vec![];
    while let Some(message) = connection.next().await {
        info!("Message received");
        let message = message.expect("Failed to get message");
        messages.push(message);
    }
    msg_tx.send(messages).expect("Failed to send results");
}

#[tokio::test]
async fn communication() {
    let _ = env_logger::try_init();

    let (con_tx, con_rx) = futures::channel::oneshot::channel();
    let (msg_tx, msg_rx) = futures::channel::oneshot::channel();

    let f = async move {
        let address = "0.0.0.0:12345"
            .to_socket_addrs()
            .expect("Not a valid address")
            .next()
            .expect("No address resolved");
        let listener = TcpListener::bind(&address).await.unwrap();
        let mut connections = listener.incoming();
        info!("Server ready");
        con_tx.send(()).unwrap();
        info!("Waiting on next connection");
        let connection = connections.next().await.expect("No connections to accept");
        let connection = connection.expect("Failed to accept connection");
        let stream = accept_async(connection).await;
        let stream = stream.expect("Failed to handshake with connection");
        run_connection(stream, msg_tx).await;
    };

    tokio::spawn(f);

    info!("Waiting for server to be ready");

    con_rx.await.expect("Server not ready");
    let address = "0.0.0.0:12345"
        .to_socket_addrs()
        .expect("Not a valid address")
        .next()
        .expect("No address resolved");
    let tcp = TcpStream::connect(&address)
        .await
        .expect("Failed to connect");
    let url = url::Url::parse("ws://localhost:12345/").unwrap();
    let (mut stream, _) = client_async(url, tcp)
        .await
        .expect("Client failed to connect");

    for i in 1..10 {
        info!("Sending message");
        stream.send(Message::Text(format!("{}", i))).await.expect("Failed to send message");
    }

    stream.close(None).await.expect("Failed to close");

    info!("Waiting for response messages");
    let messages = msg_rx.await.expect("Failed to receive messages");
    assert_eq!(messages.len(), 10);
}
