use std::io;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};

use async_tungstenite::WebSocketStream;
use futures::prelude::*;
use tungstenite::protocol::{Role, WebSocketConfig};
use tungstenite::Message;

/// A peer that sent `incoming` and accepts no writes until `open` is set.
#[derive(Clone, Default)]
struct Peer(Arc<Mutex<PeerState>>);

#[derive(Default)]
struct PeerState {
    incoming: Vec<u8>,
    open: bool,
    written: Vec<u8>,
}

impl AsyncRead for Peer {
    fn poll_read(
        self: Pin<&mut Self>,
        _: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        let mut state = self.0.lock().unwrap();
        if state.incoming.is_empty() {
            return Poll::Pending;
        }
        let n = buf.len().min(state.incoming.len());
        buf[..n].copy_from_slice(&state.incoming[..n]);
        state.incoming.drain(..n);
        Poll::Ready(Ok(n))
    }
}

impl AsyncWrite for Peer {
    fn poll_write(
        self: Pin<&mut Self>,
        _: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        let mut state = self.0.lock().unwrap();
        if !state.open {
            return Poll::Pending;
        }
        state.written.extend_from_slice(buf);
        Poll::Ready(Ok(buf.len()))
    }

    fn poll_flush(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<io::Result<()>> {
        if self.0.lock().unwrap().open {
            Poll::Ready(Ok(()))
        } else {
            Poll::Pending
        }
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        self.poll_flush(cx)
    }
}

/// A stream that read `PINGS` pings while its peer accepts no writes, so their pongs fill the
/// `LIMIT`-byte write buffer.
async fn after_unanswered_pings() -> (Peer, WebSocketStream<Peer>) {
    let peer = Peer::default();
    // Masked pings with empty payloads; each is answered by a 2-byte pong.
    peer.0.lock().unwrap().incoming = [0x89, 0x80, 0, 0, 0, 0].repeat(PINGS);
    let config = WebSocketConfig::default()
        .write_buffer_size(0)
        .max_write_buffer_size(LIMIT);
    let mut ws = WebSocketStream::from_raw_socket(peer.clone(), Role::Server, Some(config)).await;
    let mut pings = 0;
    while let Some(message) = ws.next().now_or_never() {
        assert!(
            matches!(message, Some(Ok(Message::Ping(_)))),
            "{:?}",
            message
        );
        pings += 1;
    }
    assert_eq!(pings, PINGS);
    assert!(
        peer.0.lock().unwrap().incoming.is_empty(),
        "every ping is read"
    );
    (peer, ws)
}

/// Checks `written` holds `frame` whole and nothing else but pongs.
fn around(written: &[u8], frame: &[u8]) {
    let at = written
        .windows(frame.len())
        .position(|w| w == frame)
        .expect("the frame arrives whole");
    let pongs = [&written[..at], &written[at + frame.len()..]].concat();
    assert!(!pongs.is_empty() && pongs.chunks(2).all(|p| p == [0x8A, 0]));
}

const LIMIT: usize = 64;
const PINGS: usize = 100;

#[test]
fn a_send_after_reading_pings_waits_for_their_pongs_instead_of_failing() {
    futures::executor::block_on(async {
        let (peer, mut ws) = after_unanswered_pings().await;
        let payload = vec![7; LIMIT - 2];
        let mut send = std::pin::pin!(ws.send(Message::binary(payload.clone())));
        assert!(
            send.as_mut().now_or_never().is_none(),
            "the send waits for the queued pongs to be written"
        );
        peer.0.lock().unwrap().open = true;
        send.await
            .expect("the send completes once the pongs are written");
        let written = std::mem::take(&mut peer.0.lock().unwrap().written);
        around(
            &written,
            &[&[0x82, payload.len() as u8][..], &payload].concat(),
        );
    });
}

#[cfg(feature = "futures-03-sink")]
#[test]
fn a_close_after_reading_pings_waits_for_their_pongs_instead_of_failing() {
    futures::executor::block_on(async {
        let (peer, mut ws) = after_unanswered_pings().await;
        let mut close = std::pin::pin!(futures::SinkExt::close(&mut ws));
        assert!(
            close.as_mut().now_or_never().is_none(),
            "the close waits for the queued pongs to be written"
        );
        peer.0.lock().unwrap().open = true;
        close
            .await
            .expect("the close completes once the pongs are written");
        let written = std::mem::take(&mut peer.0.lock().unwrap().written);
        around(&written, &[0x88, 0]);
    });
}
