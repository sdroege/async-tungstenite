use async_std::task;
use async_std::net::{TcpListener, TcpStream, ToSocketAddrs};
use async_tungstenite::{accept_async, client_async};

#[async_std::test]
async fn handshakes() {
    let (tx, rx) = futures::channel::oneshot::channel();

    let f = async move {
        let address = "0.0.0.0:12345"
            .to_socket_addrs()
            .await
            .expect("Not a valid address")
            .next()
            .expect("No address resolved");
        let listener = TcpListener::bind(&address).await.unwrap();
        tx.send(()).unwrap();
        while let Ok((connection, _)) = listener.accept().await {
            let stream = accept_async(connection).await;
            stream.expect("Failed to handshake with connection");
        }
    };

    task::spawn(f);

    rx.await.expect("Failed to wait for server to be ready");
    let address = "0.0.0.0:12345"
        .to_socket_addrs()
        .await
        .expect("Not a valid address")
        .next()
        .expect("No address resolved");
    let tcp = TcpStream::connect(&address)
        .await
        .expect("Failed to connect");
    let url = url::Url::parse("ws://localhost:12345/").unwrap();
    let _stream = client_async(url, tcp)
        .await
        .expect("Client failed to connect");
}
