//! Connection helper.
use tungstenite::client::url_mode;
use tungstenite::handshake::client::Response;
use tungstenite::protocol::WebSocketConfig;
use tungstenite::Error;

use futures::io::{AsyncRead, AsyncWrite};

use super::{client_async_with_config, Request, WebSocketStream};

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
    #[cfg(feature = "tls")]
    pub type Connector = async_tls::TlsConnector;
    #[cfg(feature = "native-tls")]
    pub type Connector = real_native_tls::TlsConnector;

    pub async fn wrap_stream<S>(
        socket: S,
        domain: String,
        connector: Option<Connector>,
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
                    let connector = connector.unwrap_or_else(AsyncTlsConnector::new);
                    connector.connect(&domain, socket)?.await?
                };
                #[cfg(feature = "native-tls")]
                let stream = {
                    let connector = if let Some(connector) = connector {
                        connector
                    } else {
                        let builder = real_native_tls::TlsConnector::builder();
                        builder.build()?
                    };
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

    use tungstenite::stream::Mode;
    use tungstenite::Error;

    pub type AutoStream<S> = S;
    pub type Connector = ();

    pub(crate) async fn wrap_stream<S>(
        socket: S,
        _domain: String,
        _connector: Option<()>,
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

use self::encryption::AutoStream;

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
    client_async_tls_with_connector_and_config(request, stream, None, None).await
}

/// Creates a WebSocket handshake from a request and a stream,
/// upgrading the stream to TLS if required and using the given
/// WebSocket configuration.
pub async fn client_async_tls_with_config<R, S>(
    request: R,
    stream: S,
    config: Option<WebSocketConfig>,
) -> Result<(WebSocketStream<AutoStream<S>>, Response), Error>
where
    R: Into<Request<'static>> + Unpin,
    S: 'static + AsyncRead + AsyncWrite + Send + Unpin,
    AutoStream<S>: Unpin,
{
    client_async_tls_with_connector_and_config(request, stream, None, config).await
}

/// Creates a WebSocket handshake from a request and a stream,
/// upgrading the stream to TLS if required and using the given
/// connector.
pub async fn client_async_tls_with_connector<R, S>(
    request: R,
    stream: S,
    connector: Option<self::encryption::Connector>,
) -> Result<(WebSocketStream<AutoStream<S>>, Response), Error>
where
    R: Into<Request<'static>> + Unpin,
    S: 'static + AsyncRead + AsyncWrite + Send + Unpin,
    AutoStream<S>: Unpin,
{
    client_async_tls_with_connector_and_config(request, stream, connector, None).await
}

/// Creates a WebSocket handshake from a request and a stream,
/// upgrading the stream to TLS if required and using the given
/// connector and WebSocket configuration.
pub async fn client_async_tls_with_connector_and_config<R, S>(
    request: R,
    stream: S,
    connector: Option<self::encryption::Connector>,
    config: Option<WebSocketConfig>,
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

    let stream = self::encryption::wrap_stream(stream, domain, connector, mode).await?;
    client_async_with_config(request, stream, config).await
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
        connect_async_with_config(request, None).await
    }

    /// Connect to a given URL with a given WebSocket configuration.
    pub async fn connect_async_with_config<R>(
        request: R,
        config: Option<WebSocketConfig>,
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
        client_async_tls_with_config(request, socket, config).await
    }

    #[cfg(any(feature = "tls", feature = "native-tls"))]
    /// Connect to a given URL using the provided TLS connector.
    pub async fn connect_async_with_tls_connector<R>(
        request: R,
        connector: Option<super::encryption::Connector>,
    ) -> Result<(WebSocketStream<AutoStream<TcpStream>>, Response), Error>
    where
        R: Into<Request<'static>> + Unpin,
    {
        connect_async_with_tls_connector_and_config(request, connector, None).await
    }

    #[cfg(any(feature = "tls", feature = "native-tls"))]
    /// Connect to a given URL using the provided TLS connector.
    pub async fn connect_async_with_tls_connector_and_config<R>(
        request: R,
        connector: Option<super::encryption::Connector>,
        config: Option<WebSocketConfig>,
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
        client_async_tls_with_connector_and_config(request, socket, connector, config).await
    }
}

#[cfg(feature = "async_std_runtime")]
pub use async_std_runtime::{connect_async, connect_async_with_config};
#[cfg(all(
    feature = "async_std_runtime",
    any(feature = "tls", feature = "native-tls")
))]
pub use async_std_runtime::{
    connect_async_with_tls_connector, connect_async_with_tls_connector_and_config,
};
