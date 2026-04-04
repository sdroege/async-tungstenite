#![cfg(feature = "handshake")]

use async_tungstenite::{accept_async, client_async};
use smol::net::{TcpListener, TcpStream};

#[test]
fn handshakes() {
    let test = async {
        let (tx, rx) = futures::channel::oneshot::channel();

        let f = async move {
            let listener = TcpListener::bind("127.0.0.1:12345").await.unwrap();
            tx.send(()).unwrap();
            while let Ok((connection, _)) = listener.accept().await {
                let stream = accept_async(connection).await;
                stream.expect("Failed to handshake with connection");
            }
        };

        smol::spawn(f).detach();

        rx.await.expect("Failed to wait for server to be ready");
        let tcp = TcpStream::connect("127.0.0.1:12345")
            .await
            .expect("Failed to connect");
        let url = url::Url::parse("ws://localhost:12345/").unwrap();
        let _stream = client_async(url, tcp)
            .await
            .expect("Client failed to connect");
    };
    smol::block_on(test);
}
