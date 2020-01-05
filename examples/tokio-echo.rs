use async_tungstenite::{tokio::connect_async, tungstenite::Message};
use futures::prelude::*;

async fn run() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(any(feature = "async-tls", feature = "tokio-tls"))]
    let url = url::Url::parse("wss://echo.websocket.org").unwrap();
    #[cfg(not(any(feature = "async-tls", feature = "tokio-tls")))]
    let url = url::Url::parse("ws://echo.websocket.org").unwrap();

    let (mut ws_stream, _) = connect_async(url).await?;

    let text = "Hello, World!";

    println!("Sending: \"{}\"", text);
    ws_stream.send(Message::text(text)).await?;

    let msg = ws_stream
        .next()
        .await
        .ok_or_else(|| "didn't receive anything")??;

    println!("Received: {:?}", msg);

    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut rt = tokio::runtime::Runtime::new()?;
    rt.block_on(run())
}
