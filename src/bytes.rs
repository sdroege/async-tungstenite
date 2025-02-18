//! Provides abstractions to use `AsyncRead` and `AsyncWrite` with a `WebSocketStream`.

use std::{
    io,
    pin::Pin,
    task::{Context, Poll},
};

use futures_core::stream::Stream;

use crate::{tungstenite::Bytes, Message, WsError};

/// Treat a `WebSocketStream` as an `AsyncWrite` implementation.
///
/// Every write sends a binary message. If you want to group writes together, consider wrapping
/// this with a `BufWriter`.
#[cfg(feature = "futures-03-sink")]
#[derive(Debug)]
pub struct ByteWriter<S>(S);

#[cfg(feature = "futures-03-sink")]
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

#[cfg(feature = "futures-03-sink")]
fn poll_write_helper<S>(
    mut s: Pin<&mut ByteWriter<S>>,
    cx: &mut Context<'_>,
    buf: &[u8],
) -> Poll<io::Result<usize>>
where
    S: futures_util::Sink<Message, Error = WsError> + Unpin,
{
    match Pin::new(&mut s.0).poll_ready(cx).map_err(convert_err) {
        Poll::Ready(Ok(())) => {}
        Poll::Ready(Err(e)) => return Poll::Ready(Err(e)),
        Poll::Pending => return Poll::Pending,
    }
    let len = buf.len();
    let msg = Message::binary(buf.to_owned());
    Poll::Ready(
        Pin::new(&mut s.0)
            .start_send(msg)
            .map_err(convert_err)
            .map(|()| len),
    )
}

#[cfg(feature = "futures-03-sink")]
impl<S> futures_io::AsyncWrite for ByteWriter<S>
where
    S: futures_util::Sink<Message, Error = WsError> + Unpin,
{
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        poll_write_helper(self, cx, buf)
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.0).poll_flush(cx).map_err(convert_err)
    }

    fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.0).poll_close(cx).map_err(convert_err)
    }
}

#[cfg(feature = "futures-03-sink")]
#[cfg(feature = "tokio-runtime")]
impl<S> tokio::io::AsyncWrite for ByteWriter<S>
where
    S: futures_util::Sink<Message, Error = WsError> + Unpin,
{
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        poll_write_helper(self, cx, buf)
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.0).poll_flush(cx).map_err(convert_err)
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.0).poll_close(cx).map_err(convert_err)
    }
}

/// Treat a `WebSocketStream` as an `AsyncRead` implementation.
///
/// This also works with any other `Stream` of `Message`, such as a `SplitStream`.
///
/// Each read will only return data from one message. If you want to combine data from multiple
/// messages into one read, consider wrapping this in a `BufReader`.
#[derive(Debug)]
pub struct ByteReader<S> {
    stream: S,
    bytes: Option<Bytes>,
}

impl<S> ByteReader<S> {
    /// Create a new `ByteReader` from a `Stream` that returns a WebSocket `Message`
    #[inline(always)]
    pub fn new(stream: S) -> Self {
        Self {
            stream,
            bytes: None,
        }
    }
}

impl<S> futures_io::AsyncRead for ByteReader<S>
where
    S: Stream<Item = Result<Message, WsError>> + Unpin,
{
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        let buf_len = buf.len();
        let bytes_to_read = match self.bytes {
            None => match Pin::new(&mut self.stream).poll_next(cx) {
                Poll::Pending => return Poll::Pending,
                Poll::Ready(None) => return Poll::Ready(Ok(0)),
                Poll::Ready(Some(Err(e))) => return Poll::Ready(Err(convert_err(e))),
                Poll::Ready(Some(Ok(msg))) => {
                    let bytes = msg.into_data();
                    if bytes.len() > buf_len {
                        self.bytes.insert(bytes).split_to(buf_len)
                    } else {
                        bytes
                    }
                }
            },
            Some(ref mut bytes) if bytes.len() > buf_len => bytes.split_to(buf_len),
            Some(ref mut bytes) => {
                let bytes = bytes.clone();
                self.bytes = None;
                bytes
            }
        };
        buf.copy_from_slice(&bytes_to_read);
        Poll::Ready(Ok(bytes_to_read.len()))
    }
}

fn convert_err(e: WsError) -> io::Error {
    match e {
        WsError::Io(io) => io,
        _ => io::Error::new(io::ErrorKind::Other, e),
    }
}
