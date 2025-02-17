//! Provides an abstraction to use `AsyncWrite` to write bytes to a `WebSocketStream`.

use std::{
    io,
    pin::Pin,
    task::{Context, Poll},
};

use futures_util::Sink;

use crate::{Message, WsError};

/// Treat a `WebSocketStream` as an `AsyncWrite` implementation.
///
/// Every write sends a binary message. If you want to group writes together, consider wrapping
/// this with a `BufWriter`.
#[derive(Debug)]
pub struct ByteWriter<S>(S);

impl<S> ByteWriter<S> {
    /// Create a new `ByteWriter` from a `Sink` that accepts a WebSocket `Message`
    #[inline(always)]
    pub fn new(s: S) -> Self {
        Self(s)
    }

    /// Get the underlying `Sink` back.
    #[inline(always)]
    pub fn into_inner(self) -> S {
        self.0
    }
}

impl<S> futures_io::AsyncWrite for ByteWriter<S>
where
    S: Sink<Message, Error = WsError> + Unpin,
{
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        match Pin::new(&mut self.0).poll_ready(cx).map_err(convert_err) {
            Poll::Ready(Ok(())) => {}
            Poll::Ready(Err(e)) => return Poll::Ready(Err(e)),
            Poll::Pending => return Poll::Pending,
        }
        let len = buf.len();
        let msg = Message::binary(buf.to_owned());
        Poll::Ready(
            Pin::new(&mut self.0)
                .start_send(msg)
                .map_err(convert_err)
                .map(|()| len),
        )
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.0).poll_flush(cx).map_err(convert_err)
    }

    fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.0).poll_close(cx).map_err(convert_err)
    }
}

fn convert_err(e: WsError) -> io::Error {
    match e {
        WsError::Io(io) => io,
        _ => io::Error::new(io::ErrorKind::Other, e),
    }
}
