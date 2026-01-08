use async_tungstenite::{smol::connect_async, tungstenite::Message};
use futures::prelude::*;

async fn run() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(any(feature = "async-tls", feature = "async-native-tls"))]
    let url = "wss://echo.websocket.org/.ws";
    #[cfg(not(any(feature = "async-tls", feature = "async-native-tls")))]
    let url = "ws://echo.websocket.org/.ws";

    println!("Connecting: \"{}\"", url);
    let (mut ws_stream, _) = connect_async(url).await?;

    let msg = ws_stream.next().await.ok_or("didn't receive anything")??;
    println!("Received: {:?}", msg);

    let text = "Hello, World!";

    println!("Sending: \"{}\"", text);
    ws_stream.send(Message::text(text)).await?;

    let msg = ws_stream.next().await.ok_or("didn't receive anything")??;

    println!("Received: {:?}", msg);

    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    smol::block_on(run())
}
