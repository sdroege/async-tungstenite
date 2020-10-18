//! `tokio` integration.
use tungstenite::client::IntoClientRequest;
use tungstenite::handshake::client::{Request, Response};
use tungstenite::handshake::server::{Callback, NoCallback};
use tungstenite::protocol::WebSocketConfig;
use tungstenite::Error;

use tokio::net::TcpStream;

use super::{domain, port, WebSocketStream};

use futures_io::{AsyncRead, AsyncWrite};

#[cfg(all(feature = "tokio-rustls", not(feature = "tokio-native-tls")))]
pub(crate) mod tokio_tls {
    use real_tokio_rustls::rustls::ClientConfig;
    use real_tokio_rustls::webpki::DNSNameRef;
    use real_tokio_rustls::{client::TlsStream, TlsConnector as AsyncTlsConnector};

    use tungstenite::client::{uri_mode, IntoClientRequest};
    use tungstenite::handshake::client::Request;
    use tungstenite::stream::Mode;
    use tungstenite::Error;

    use crate::stream::Stream as StreamSwitcher;
    use crate::{client_async_with_config, domain, Response, WebSocketConfig, WebSocketStream};

    use super::TokioAdapter;

    /// A stream that might be protected with TLS.
    pub type MaybeTlsStream<S> = StreamSwitcher<TokioAdapter<S>, TokioAdapter<TlsStream<S>>>;

    pub type AutoStream<S> = MaybeTlsStream<S>;

    pub type Connector = AsyncTlsConnector;

    async fn wrap_stream<S>(
        socket: S,
        domain: String,
        connector: Option<Connector>,
        mode: Mode,
    ) -> Result<AutoStream<S>, Error>
    where
        S: 'static + tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
    {
        match mode {
            Mode::Plain => Ok(StreamSwitcher::Plain(TokioAdapter(socket))),
            Mode::Tls => {
                let stream = {
                    let connector = if let Some(connector) = connector {
                        connector
                    } else {
                        let mut config = ClientConfig::new();
                        config
                            .root_store
                            .add_server_trust_anchors(&webpki_roots::TLS_SERVER_ROOTS);
                        AsyncTlsConnector::from(std::sync::Arc::new(config))
                    };
                    let domain = DNSNameRef::try_from_ascii_str(&domain)
                        .map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, err))?;
                    connector
                        .connect(domain, socket)
                        .await
                        .map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, err))?
                };
                Ok(StreamSwitcher::Tls(TokioAdapter(stream)))
            }
        }
    }

    /// Creates a WebSocket handshake from a request and a stream,
    /// upgrading the stream to TLS if required and using the given
    /// connector and WebSocket configuration.
    pub async fn client_async_tls_with_connector_and_config<R, S>(
        request: R,
        stream: S,
        connector: Option<AsyncTlsConnector>,
        config: Option<WebSocketConfig>,
    ) -> Result<(WebSocketStream<AutoStream<S>>, Response), Error>
    where
        R: IntoClientRequest + Unpin,
        S: 'static + tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
        AutoStream<S>: Unpin,
    {
        let request: Request = request.into_client_request()?;

        let domain = domain(&request)?;

        // Make sure we check domain and mode first. URL must be valid.
        let mode = uri_mode(request.uri())?;

        let stream = wrap_stream(stream, domain, connector, mode).await?;
        client_async_with_config(request, stream, config).await
    }
}

#[cfg(feature = "tokio-native-tls")]
pub(crate) mod tokio_tls {
    use real_tokio_native_tls::TlsConnector as AsyncTlsConnector;
    use real_tokio_native_tls::TlsStream;

    use tungstenite::client::{uri_mode, IntoClientRequest};
    use tungstenite::handshake::client::Request;
    use tungstenite::stream::Mode;
    use tungstenite::Error;

    use crate::stream::Stream as StreamSwitcher;
    use crate::{client_async_with_config, domain, Response, WebSocketConfig, WebSocketStream};

    use super::TokioAdapter;

    /// A stream that might be protected with TLS.
    pub type MaybeTlsStream<S> = StreamSwitcher<TokioAdapter<S>, TokioAdapter<TlsStream<S>>>;

    pub type AutoStream<S> = MaybeTlsStream<S>;

    pub type Connector = AsyncTlsConnector;

    async fn wrap_stream<S>(
        socket: S,
        domain: String,
        connector: Option<Connector>,
        mode: Mode,
    ) -> Result<AutoStream<S>, Error>
    where
        S: 'static + tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
    {
        match mode {
            Mode::Plain => Ok(StreamSwitcher::Plain(TokioAdapter(socket))),
            Mode::Tls => {
                let stream = {
                    let connector = if let Some(connector) = connector {
                        connector
                    } else {
                        let connector = real_native_tls::TlsConnector::builder().build()?;
                        AsyncTlsConnector::from(connector)
                    };
                    connector
                        .connect(&domain, socket)
                        .await
                        .map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, err))?
                };
                Ok(StreamSwitcher::Tls(TokioAdapter(stream)))
            }
        }
    }

    /// Creates a WebSocket handshake from a request and a stream,
    /// upgrading the stream to TLS if required and using the given
    /// connector and WebSocket configuration.
    pub async fn client_async_tls_with_connector_and_config<R, S>(
        request: R,
        stream: S,
        connector: Option<AsyncTlsConnector>,
        config: Option<WebSocketConfig>,
    ) -> Result<(WebSocketStream<AutoStream<S>>, Response), Error>
    where
        R: IntoClientRequest + Unpin,
        S: 'static + tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
        AutoStream<S>: Unpin,
    {
        let request: Request = request.into_client_request()?;

        let domain = domain(&request)?;

        // Make sure we check domain and mode first. URL must be valid.
        let mode = uri_mode(request.uri())?;

        let stream = wrap_stream(stream, domain, connector, mode).await?;
        client_async_with_config(request, stream, config).await
    }
}

#[cfg(all(
    feature = "tokio-openssl",
    not(any(feature = "tokio-native-tls", feature = "tokio-rustls"))
))]
pub(crate) mod tokio_tls {
    use openssl::ssl::{ConnectConfiguration, SslConnector, SslMethod};
    use real_tokio_openssl::connect;
    use real_tokio_openssl::SslStream as TlsStream;

    use tungstenite::client::{uri_mode, IntoClientRequest};
    use tungstenite::handshake::client::Request;
    use tungstenite::stream::Mode;
    use tungstenite::Error;

    use crate::stream::Stream as StreamSwitcher;
    use crate::{client_async_with_config, domain, Response, WebSocketConfig, WebSocketStream};

    use super::TokioAdapter;

    /// A stream that might be protected with TLS.
    pub type MaybeTlsStream<S> = StreamSwitcher<TokioAdapter<S>, TokioAdapter<TlsStream<S>>>;

    pub type AutoStream<S> = MaybeTlsStream<S>;

    pub type Connector = ConnectConfiguration;

    async fn wrap_stream<S>(
        socket: S,
        domain: String,
        connector: Option<Connector>,
        mode: Mode,
    ) -> Result<AutoStream<S>, Error>
    where
        S: 'static
            + tokio::io::AsyncRead
            + tokio::io::AsyncWrite
            + Unpin
            + std::fmt::Debug
            + Send
            + Sync,
    {
        match mode {
            Mode::Plain => Ok(StreamSwitcher::Plain(TokioAdapter(socket))),
            Mode::Tls => {
                let stream = {
                    let connector = if let Some(connector) = connector {
                        connector
                    } else {
                        SslConnector::builder(SslMethod::tls_client())
                            .map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, err))?
                            .build()
                            .configure()
                            .map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, err))?
                    };
                    connect(connector, &domain, socket)
                        .await
                        .map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, err))?
                };
                Ok(StreamSwitcher::Tls(TokioAdapter(stream)))
            }
        }
    }

    /// Creates a WebSocket handshake from a request and a stream,
    /// upgrading the stream to TLS if required and using the given
    /// connector and WebSocket configuration.
    pub async fn client_async_tls_with_connector_and_config<R, S>(
        request: R,
        stream: S,
        connector: Option<Connector>,
        config: Option<WebSocketConfig>,
    ) -> Result<(WebSocketStream<AutoStream<S>>, Response), Error>
    where
        R: IntoClientRequest + Unpin,
        S: 'static
            + tokio::io::AsyncRead
            + tokio::io::AsyncWrite
            + Unpin
            + std::fmt::Debug
            + Send
            + Sync,
        AutoStream<S>: Unpin,
    {
        let request: Request = request.into_client_request()?;

        let domain = domain(&request)?;

        // Make sure we check domain and mode first. URL must be valid.
        let mode = uri_mode(request.uri())?;

        let stream = wrap_stream(stream, domain, connector, mode).await?;
        client_async_with_config(request, stream, config).await
    }
}

#[cfg(not(any(
    feature = "async-tls",
    feature = "tokio-native-tls",
    feature = "tokio-rustls",
    feature = "tokio-openssl"
)))]
pub(crate) mod dummy_tls {
    use tungstenite::client::{uri_mode, IntoClientRequest};
    use tungstenite::handshake::client::Request;
    use tungstenite::stream::Mode;
    use tungstenite::Error;

    use super::TokioAdapter;

    use crate::{client_async_with_config, domain, Response, WebSocketConfig, WebSocketStream};

    pub type AutoStream<S> = TokioAdapter<S>;
    type Connector = ();

    async fn wrap_stream<S>(
        socket: S,
        _domain: String,
        _connector: Option<()>,
        mode: Mode,
    ) -> Result<AutoStream<S>, Error>
    where
        S: 'static + tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
    {
        match mode {
            Mode::Plain => Ok(TokioAdapter(socket)),
            Mode::Tls => Err(Error::Url("TLS support not compiled in.".into())),
        }
    }

    pub(crate) async fn client_async_tls_with_connector_and_config<R, S>(
        request: R,
        stream: S,
        connector: Option<Connector>,
        config: Option<WebSocketConfig>,
    ) -> Result<(WebSocketStream<AutoStream<S>>, Response), Error>
    where
        R: IntoClientRequest + Unpin,
        S: 'static + tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
        AutoStream<S>: Unpin,
    {
        let request: Request = request.into_client_request()?;

        let domain = domain(&request)?;

        // Make sure we check domain and mode first. URL must be valid.
        let mode = uri_mode(request.uri())?;

        let stream = wrap_stream(stream, domain, connector, mode).await?;
        client_async_with_config(request, stream, config).await
    }
}

#[cfg(not(any(
    feature = "async-tls",
    feature = "tokio-native-tls",
    feature = "tokio-rustls",
    feature = "tokio-openssl"
)))]
use self::dummy_tls::{client_async_tls_with_connector_and_config, AutoStream};

#[cfg(all(
    feature = "async-tls",
    not(any(
        feature = "tokio-rustls",
        feature = "tokio-native-tls",
        feature = "tokio-openssl"
    ))
))]
pub(crate) mod async_tls_adapter {
    use super::{
        Error, IntoClientRequest, Response, TokioAdapter, WebSocketConfig, WebSocketStream,
    };
    use crate::stream::Stream as StreamSwitcher;
    use std::marker::Unpin;

    /// Creates a WebSocket handshake from a request and a stream,
    /// upgrading the stream to TLS if required and using the given
    /// connector and WebSocket configuration.
    pub async fn client_async_tls_with_connector_and_config<R, S>(
        request: R,
        stream: S,
        connector: Option<Connector>,
        config: Option<WebSocketConfig>,
    ) -> Result<(WebSocketStream<AutoStream<S>>, Response), Error>
    where
        R: IntoClientRequest + Unpin,
        S: 'static + tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
        AutoStream<S>: Unpin,
    {
        crate::async_tls::client_async_tls_with_connector_and_config(
            request,
            TokioAdapter(stream),
            connector,
            config,
        )
        .await
    }

    pub type Connector = real_async_tls::TlsConnector;

    pub type MaybeTlsStream<S> = StreamSwitcher<S, real_async_tls::client::TlsStream<S>>;

    pub type AutoStream<S> = MaybeTlsStream<TokioAdapter<S>>;
}

#[cfg(all(
    feature = "async-tls",
    not(any(
        feature = "tokio-rustls",
        feature = "tokio-native-tls",
        feature = "tokio-openssl"
    ))
))]
pub use self::async_tls_adapter::client_async_tls_with_connector_and_config;
#[cfg(all(
    feature = "async-tls",
    not(any(
        feature = "tokio-rustls",
        feature = "tokio-native-tls",
        feature = "tokio-openssl"
    ))
))]
use self::async_tls_adapter::{AutoStream, Connector};

#[cfg(any(
    feature = "tokio-rustls",
    feature = "tokio-native-tls",
    feature = "tokio-openssl"
))]
pub use self::tokio_tls::client_async_tls_with_connector_and_config;
#[cfg(any(
    feature = "tokio-rustls",
    feature = "tokio-native-tls",
    feature = "tokio-openssl"
))]
use self::tokio_tls::{AutoStream, Connector};

/// Creates a WebSocket handshake from a request and a stream.
/// For convenience, the user may call this with a url string, a URL,
/// or a `Request`. Calling with `Request` allows the user to add
/// a WebSocket protocol or other custom headers.
///
/// Internally, this custom creates a handshake representation and returns
/// a future representing the resolution of the WebSocket handshake. The
/// returned future will resolve to either `WebSocketStream<S>` or `Error`
/// depending on whether the handshake is successful.
///
/// This is typically used for clients who have already established, for
/// example, a TCP connection to the remote server.
pub async fn client_async<'a, R, S>(
    request: R,
    stream: S,
) -> Result<(WebSocketStream<TokioAdapter<S>>, Response), Error>
where
    R: IntoClientRequest + Unpin,
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    client_async_with_config(request, stream, None).await
}

/// The same as `client_async()` but the one can specify a websocket configuration.
/// Please refer to `client_async()` for more details.
pub async fn client_async_with_config<'a, R, S>(
    request: R,
    stream: S,
    config: Option<WebSocketConfig>,
) -> Result<(WebSocketStream<TokioAdapter<S>>, Response), Error>
where
    R: IntoClientRequest + Unpin,
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    crate::client_async_with_config(request, TokioAdapter(stream), config).await
}

/// Accepts a new WebSocket connection with the provided stream.
///
/// This function will internally call `server::accept` to create a
/// handshake representation and returns a future representing the
/// resolution of the WebSocket handshake. The returned future will resolve
/// to either `WebSocketStream<S>` or `Error` depending if it's successful
/// or not.
///
/// This is typically used after a socket has been accepted from a
/// `TcpListener`. That socket is then passed to this function to perform
/// the server half of the accepting a client's websocket connection.
pub async fn accept_async<S>(stream: S) -> Result<WebSocketStream<TokioAdapter<S>>, Error>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    accept_hdr_async(stream, NoCallback).await
}

/// The same as `accept_async()` but the one can specify a websocket configuration.
/// Please refer to `accept_async()` for more details.
pub async fn accept_async_with_config<S>(
    stream: S,
    config: Option<WebSocketConfig>,
) -> Result<WebSocketStream<TokioAdapter<S>>, Error>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    accept_hdr_async_with_config(stream, NoCallback, config).await
}

/// Accepts a new WebSocket connection with the provided stream.
///
/// This function does the same as `accept_async()` but accepts an extra callback
/// for header processing. The callback receives headers of the incoming
/// requests and is able to add extra headers to the reply.
pub async fn accept_hdr_async<S, C>(
    stream: S,
    callback: C,
) -> Result<WebSocketStream<TokioAdapter<S>>, Error>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
    C: Callback + Unpin,
{
    accept_hdr_async_with_config(stream, callback, None).await
}

/// The same as `accept_hdr_async()` but the one can specify a websocket configuration.
/// Please refer to `accept_hdr_async()` for more details.
pub async fn accept_hdr_async_with_config<S, C>(
    stream: S,
    callback: C,
    config: Option<WebSocketConfig>,
) -> Result<WebSocketStream<TokioAdapter<S>>, Error>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
    C: Callback + Unpin,
{
    crate::accept_hdr_async_with_config(TokioAdapter(stream), callback, config).await
}

/// Type alias for the stream type of the `client_async()` functions.
pub type ClientStream<S> = AutoStream<S>;

#[cfg(any(
    feature = "tokio-native-tls",
    feature = "tokio-rustls",
    all(feature = "async-tls", not(feature = "tokio-openssl"))
))]
/// Creates a WebSocket handshake from a request and a stream,
/// upgrading the stream to TLS if required.
pub async fn client_async_tls<R, S>(
    request: R,
    stream: S,
) -> Result<(WebSocketStream<ClientStream<S>>, Response), Error>
where
    R: IntoClientRequest + Unpin,
    S: 'static + tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
    AutoStream<S>: Unpin,
{
    client_async_tls_with_connector_and_config(request, stream, None, None).await
}

#[cfg(any(
    feature = "tokio-native-tls",
    feature = "tokio-rustls",
    all(feature = "async-tls", not(feature = "tokio-openssl"))
))]
/// Creates a WebSocket handshake from a request and a stream,
/// upgrading the stream to TLS if required and using the given
/// WebSocket configuration.
pub async fn client_async_tls_with_config<R, S>(
    request: R,
    stream: S,
    config: Option<WebSocketConfig>,
) -> Result<(WebSocketStream<ClientStream<S>>, Response), Error>
where
    R: IntoClientRequest + Unpin,
    S: 'static + tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
    AutoStream<S>: Unpin,
{
    client_async_tls_with_connector_and_config(request, stream, None, config).await
}

#[cfg(any(
    feature = "tokio-native-tls",
    feature = "tokio-rustls",
    all(feature = "async-tls", not(feature = "tokio-openssl"))
))]
/// Creates a WebSocket handshake from a request and a stream,
/// upgrading the stream to TLS if required and using the given
/// connector.
pub async fn client_async_tls_with_connector<R, S>(
    request: R,
    stream: S,
    connector: Option<Connector>,
) -> Result<(WebSocketStream<ClientStream<S>>, Response), Error>
where
    R: IntoClientRequest + Unpin,
    S: 'static + tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
    AutoStream<S>: Unpin,
{
    client_async_tls_with_connector_and_config(request, stream, connector, None).await
}

#[cfg(all(
    feature = "tokio-openssl",
    not(any(feature = "tokio-native-tls", feature = "tokio-rustls"))
))]
/// Creates a WebSocket handshake from a request and a stream,
/// upgrading the stream to TLS if required.
pub async fn client_async_tls<R, S>(
    request: R,
    stream: S,
) -> Result<(WebSocketStream<ClientStream<S>>, Response), Error>
where
    R: IntoClientRequest + Unpin,
    S: 'static
        + tokio::io::AsyncRead
        + tokio::io::AsyncWrite
        + Unpin
        + std::fmt::Debug
        + Send
        + Sync,
    AutoStream<S>: Unpin,
{
    client_async_tls_with_connector_and_config(request, stream, None, None).await
}

#[cfg(all(
    feature = "tokio-openssl",
    not(any(feature = "tokio-native-tls", feature = "tokio-rustls"))
))]
/// Creates a WebSocket handshake from a request and a stream,
/// upgrading the stream to TLS if required and using the given
/// WebSocket configuration.
pub async fn client_async_tls_with_config<R, S>(
    request: R,
    stream: S,
    config: Option<WebSocketConfig>,
) -> Result<(WebSocketStream<ClientStream<S>>, Response), Error>
where
    R: IntoClientRequest + Unpin,
    S: 'static
        + tokio::io::AsyncRead
        + tokio::io::AsyncWrite
        + Unpin
        + std::fmt::Debug
        + Send
        + Sync,
    AutoStream<S>: Unpin,
{
    client_async_tls_with_connector_and_config(request, stream, None, config).await
}

#[cfg(all(
    feature = "tokio-openssl",
    not(any(feature = "tokio-native-tls", feature = "tokio-rustls"))
))]
/// Creates a WebSocket handshake from a request and a stream,
/// upgrading the stream to TLS if required and using the given
/// connector.
pub async fn client_async_tls_with_connector<R, S>(
    request: R,
    stream: S,
    connector: Option<Connector>,
) -> Result<(WebSocketStream<ClientStream<S>>, Response), Error>
where
    R: IntoClientRequest + Unpin,
    S: 'static
        + tokio::io::AsyncRead
        + tokio::io::AsyncWrite
        + Unpin
        + std::fmt::Debug
        + Send
        + Sync,
    AutoStream<S>: Unpin,
{
    client_async_tls_with_connector_and_config(request, stream, connector, None).await
}

/// Type alias for the stream type of the `connect_async()` functions.
pub type ConnectStream = ClientStream<TcpStream>;

/// Connect to a given URL.
pub async fn connect_async<R>(
    request: R,
) -> Result<(WebSocketStream<ConnectStream>, Response), Error>
where
    R: IntoClientRequest + Unpin,
{
    connect_async_with_config(request, None).await
}

/// Connect to a given URL with a given WebSocket configuration.
pub async fn connect_async_with_config<R>(
    request: R,
    config: Option<WebSocketConfig>,
) -> Result<(WebSocketStream<ConnectStream>, Response), Error>
where
    R: IntoClientRequest + Unpin,
{
    let request: Request = request.into_client_request()?;

    let domain = domain(&request)?;
    let port = port(&request)?;

    let try_socket = TcpStream::connect((domain.as_str(), port)).await;
    let socket = try_socket.map_err(Error::Io)?;
    client_async_tls_with_connector_and_config(request, socket, None, config).await
}

#[cfg(any(
    feature = "async-tls",
    feature = "tokio-native-tls",
    feature = "tokio-rustls",
    feature = "tokio-openssl"
))]
/// Connect to a given URL using the provided TLS connector.
pub async fn connect_async_with_tls_connector<R>(
    request: R,
    connector: Option<Connector>,
) -> Result<(WebSocketStream<ConnectStream>, Response), Error>
where
    R: IntoClientRequest + Unpin,
{
    connect_async_with_tls_connector_and_config(request, connector, None).await
}

#[cfg(any(
    feature = "async-tls",
    feature = "tokio-native-tls",
    feature = "tokio-rustls",
    feature = "tokio-openssl"
))]
/// Connect to a given URL using the provided TLS connector.
pub async fn connect_async_with_tls_connector_and_config<R>(
    request: R,
    connector: Option<Connector>,
    config: Option<WebSocketConfig>,
) -> Result<(WebSocketStream<ConnectStream>, Response), Error>
where
    R: IntoClientRequest + Unpin,
{
    let request: Request = request.into_client_request()?;

    let domain = domain(&request)?;
    let port = port(&request)?;

    let try_socket = TcpStream::connect((domain.as_str(), port)).await;
    let socket = try_socket.map_err(Error::Io)?;
    client_async_tls_with_connector_and_config(request, socket, connector, config).await
}

use pin_project::pin_project;
use std::pin::Pin;
use std::task::{Context, Poll};

/// Adapter for `tokio::io::AsyncRead` and `tokio::io::AsyncWrite` to provide
/// the variants from the `futures` crate and the other way around.
#[pin_project]
#[derive(Debug, Clone)]
pub struct TokioAdapter<T>(#[pin] pub T);

impl<T: tokio::io::AsyncRead> AsyncRead for TokioAdapter<T> {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<std::io::Result<usize>> {
        self.project().0.poll_read(cx, buf)
    }
}

impl<T: tokio::io::AsyncWrite> AsyncWrite for TokioAdapter<T> {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, std::io::Error>> {
        self.project().0.poll_write(cx, buf)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), std::io::Error>> {
        self.project().0.poll_flush(cx)
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), std::io::Error>> {
        self.project().0.poll_shutdown(cx)
    }
}

impl<T: AsyncRead> tokio::io::AsyncRead for TokioAdapter<T> {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<std::io::Result<usize>> {
        self.project().0.poll_read(cx, buf)
    }
}

impl<T: AsyncWrite> tokio::io::AsyncWrite for TokioAdapter<T> {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, std::io::Error>> {
        self.project().0.poll_write(cx, buf)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), std::io::Error>> {
        self.project().0.poll_flush(cx)
    }

    fn poll_shutdown(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Result<(), std::io::Error>> {
        self.project().0.poll_close(cx)
    }
}
