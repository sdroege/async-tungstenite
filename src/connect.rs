//! Connection helper.
use tungstenite::client::url_mode;
use tungstenite::handshake::client::Response;
use tungstenite::Error;

use futures::io::{AsyncRead, AsyncWrite};

use super::{client_async, Request, WebSocketStream};

#[cfg(feature = "tls-base")]
pub(crate) mod encryption {
    #[cfg(feature = "tls")]
    use async_tls::client::TlsStream;
    #[cfg(feature = "tls")]
    use async_tls::TlsConnector as AsyncTlsConnector;

    #[cfg(feature = "native-tls")]
    use async_native_tls::TlsConnector as AsyncTlsConnector;
    #[cfg(feature = "native-tls")]
    use async_native_tls::TlsStream;

    use tungstenite::stream::Mode;
    use tungstenite::Error;

    use futures::io::{AsyncRead, AsyncWrite};

    use crate::stream::Stream as StreamSwitcher;

    /// A stream that might be protected with TLS.
    pub type MaybeTlsStream<S> = StreamSwitcher<S, TlsStream<S>>;

    pub type AutoStream<S> = MaybeTlsStream<S>;

    pub async fn wrap_stream<S>(
        socket: S,
        domain: String,
        mode: Mode,
    ) -> Result<AutoStream<S>, Error>
    where
        S: 'static + AsyncRead + AsyncWrite + Send + Unpin,
    {
        match mode {
            Mode::Plain => Ok(StreamSwitcher::Plain(socket)),
            Mode::Tls => {
                #[cfg(feature = "tls")]
                let stream = {
                    let connector = AsyncTlsConnector::new();
                    connector.connect(&domain, socket)?.await?
                };
                #[cfg(feature = "native-tls")]
                let stream = {
                    let builder = real_native_tls::TlsConnector::builder();
                    let connector = builder.build()?;
                    let connector = AsyncTlsConnector::from(connector);
                    connector.connect(&domain, socket).await?
                };
                Ok(StreamSwitcher::Tls(stream))
            }
        }
    }
}

#[cfg(feature = "tls-base")]
pub use self::encryption::MaybeTlsStream;

#[cfg(not(feature = "tls-base"))]
pub(crate) mod encryption {
    use futures::io::{AsyncRead, AsyncWrite};
    use futures::{future, Future};

    use tungstenite::stream::Mode;
    use tungstenite::Error;

    pub type AutoStream<S> = S;

    pub async fn wrap_stream<S>(
        socket: S,
        _domain: String,
        mode: Mode,
    ) -> Result<AutoStream<S>, Error>
    where
        S: 'static + AsyncRead + AsyncWrite + Send + Unpin,
    {
        match mode {
            Mode::Plain => Ok(socket),
            Mode::Tls => Err(Error::Url("TLS support not compiled in.".into())),
        }
    }
}

use self::encryption::{wrap_stream, AutoStream};

/// Get a domain from an URL.
#[inline]
fn domain(request: &Request) -> Result<String, Error> {
    match request.url.host_str() {
        Some(d) => Ok(d.to_string()),
        None => Err(Error::Url("no host name in the url".into())),
    }
}

/// Creates a WebSocket handshake from a request and a stream,
/// upgrading the stream to TLS if required.
pub async fn client_async_tls<R, S>(
    request: R,
    stream: S,
) -> Result<(WebSocketStream<AutoStream<S>>, Response), Error>
where
    R: Into<Request<'static>> + Unpin,
    S: 'static + AsyncRead + AsyncWrite + Send + Unpin,
    AutoStream<S>: Unpin,
{
    let request: Request = request.into();

    let domain = domain(&request)?;

    // Make sure we check domain and mode first. URL must be valid.
    let mode = url_mode(&request.url)?;

    let stream = wrap_stream(stream, domain, mode).await?;
    client_async(request, stream).await
}

#[cfg(feature = "async_std_runtime")]
pub(crate) mod async_std_runtime {
    use super::*;
    use async_std::net::TcpStream;

    /// Connect to a given URL.
    pub async fn connect_async<R>(
        request: R,
    ) -> Result<(WebSocketStream<AutoStream<TcpStream>>, Response), Error>
    where
        R: Into<Request<'static>> + Unpin,
    {
        let request: Request = request.into();

        let domain = domain(&request)?;
        let port = request
            .url
            .port_or_known_default()
            .expect("Bug: port unknown");

        let try_socket = TcpStream::connect((domain.as_str(), port)).await;
        let socket = try_socket.map_err(Error::Io)?;
        client_async_tls(request, socket).await
    }
}

#[cfg(feature = "async_std_runtime")]
pub use async_std_runtime::connect_async;
