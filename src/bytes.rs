//! Provides abstractions to use `AsyncRead` and `AsyncWrite` with
//! a [`WebSocketStream`](crate::WebSocketStream) or a [`WebSocketSender`](crate::WebSocketSender).

use std::{
    io,
    pin::Pin,
    task::{ready, Context, Poll},
};

use futures_core::stream::Stream;

use crate::{tungstenite::Bytes, Message, WsError};

/// Treat a websocket [sender](Sender) as an `AsyncWrite` implementation.
///
/// Every write sends a binary message. If you want to group writes together, consider wrapping
/// this with a `BufWriter`.
#[derive(Debug)]
pub struct ByteWriter<S>(S);

impl<S> ByteWriter<S> {
    /// Create a new `ByteWriter` from a [sender](Sender) that accepts a websocket [`Message`].
    #[inline(always)]
    pub fn new(s: S) -> Self
    where
        S: Sender,
    {
        Self(s)
    }

    /// Get the underlying [sender](Sender) back.
    #[inline(always)]
    pub fn into_inner(self) -> S {
        self.0
    }
}

/// Sends bytes as a websocket [`Message`].
///
/// It's implemented for [`WebSocketStream`](crate::WebSocketStream)
/// and [`WebSocketSender`](crate::WebSocketSender).
/// It's also implemeted for every `Sink` type that accepts
/// a websocket [`Message`] and returns [`WsError`] type as
/// an error when `futures-03-sink` feature is enabled.
pub trait Sender: private::SealedSender {}

pub(crate) mod private {
    use super::*;

    pub trait SealedSender {
        fn poll_write(
            self: Pin<&mut Self>,
            cx: &mut Context<'_>,
            buf: &[u8],
        ) -> Poll<Result<usize, WsError>>;

        fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), WsError>>;
        fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), WsError>>;
    }

    impl<S> Sender for S where S: SealedSender {}
}

#[cfg(feature = "futures-03-sink")]
impl<S> private::SealedSender for S
where
    S: futures_util::Sink<Message, Error = WsError> + Unpin,
{
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, WsError>> {
        ready!(self.as_mut().poll_ready(cx))?;
        let len = buf.len();
        self.start_send(Message::binary(buf.to_owned()))?;
        Poll::Ready(Ok(len))
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), WsError>> {
        <S as futures_util::Sink<_>>::poll_flush(self, cx)
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), WsError>> {
        <S as futures_util::Sink<_>>::poll_close(self, cx)
    }
}

impl<S> futures_io::AsyncWrite for ByteWriter<S>
where
    S: Sender + Unpin,
{
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        <S as private::SealedSender>::poll_write(Pin::new(&mut self.0), cx, buf)
            .map_err(convert_err)
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        <S as private::SealedSender>::poll_flush(Pin::new(&mut self.0), cx).map_err(convert_err)
    }

    fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        <S as private::SealedSender>::poll_close(Pin::new(&mut self.0), cx).map_err(convert_err)
    }
}

#[cfg(feature = "tokio-runtime")]
impl<S> tokio::io::AsyncWrite for ByteWriter<S>
where
    S: Sender + Unpin,
{
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        <S as private::SealedSender>::poll_write(Pin::new(&mut self.0), cx, buf)
            .map_err(convert_err)
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        <S as private::SealedSender>::poll_flush(Pin::new(&mut self.0), cx).map_err(convert_err)
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        <S as private::SealedSender>::poll_close(Pin::new(&mut self.0), cx).map_err(convert_err)
    }
}

/// Treat a websocket [stream](Stream) as an `AsyncRead` implementation.
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
    /// Create a new `ByteReader` from a [stream](Stream) that returns a WebSocket [`Message`].
    #[inline(always)]
    pub fn new(stream: S) -> Self {
        Self {
            stream,
            bytes: None,
        }
    }
}

fn poll_read_helper<S>(
    mut s: Pin<&mut ByteReader<S>>,
    cx: &mut Context<'_>,
    buf_len: usize,
) -> Poll<io::Result<Option<Bytes>>>
where
    S: Stream<Item = Result<Message, WsError>> + Unpin,
{
    Poll::Ready(Ok(Some(match s.bytes {
        None => match Pin::new(&mut s.stream).poll_next(cx) {
            Poll::Pending => return Poll::Pending,
            Poll::Ready(None) => return Poll::Ready(Ok(None)),
            Poll::Ready(Some(Err(e))) => return Poll::Ready(Err(convert_err(e))),
            Poll::Ready(Some(Ok(msg))) => {
                let bytes = msg.into_data();
                if bytes.len() > buf_len {
                    s.bytes.insert(bytes).split_to(buf_len)
                } else {
                    bytes
                }
            }
        },
        Some(ref mut bytes) if bytes.len() > buf_len => bytes.split_to(buf_len),
        Some(ref mut bytes) => {
            let bytes = bytes.clone();
            s.bytes = None;
            bytes
        }
    })))
}

impl<S> futures_io::AsyncRead for ByteReader<S>
where
    S: Stream<Item = Result<Message, WsError>> + Unpin,
{
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        poll_read_helper(self, cx, buf.len()).map_ok(|bytes| {
            bytes.map_or(0, |bytes| {
                buf[..bytes.len()].copy_from_slice(&bytes);
                bytes.len()
            })
        })
    }
}

#[cfg(feature = "tokio-runtime")]
impl<S> tokio::io::AsyncRead for ByteReader<S>
where
    S: Stream<Item = Result<Message, WsError>> + Unpin,
{
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut tokio::io::ReadBuf,
    ) -> Poll<io::Result<()>> {
        poll_read_helper(self, cx, buf.remaining()).map_ok(|bytes| {
            if let Some(ref bytes) = bytes {
                buf.put_slice(bytes);
            }
        })
    }
}

fn convert_err(e: WsError) -> io::Error {
    match e {
        WsError::Io(io) => io,
        _ => io::Error::new(io::ErrorKind::Other, e),
    }
}
