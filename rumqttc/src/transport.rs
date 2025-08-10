use std::net::SocketAddr;
#[cfg(any(feature = "use-rustls", feature = "websocket"))]
use std::sync::Arc;
#[cfg(unix)]
use {std::path::Path, tokio::net::UnixStream};

use rumqtt_bytes::{Packet, Protocol};
use tokio::net::{lookup_host, TcpSocket, TcpStream};
#[cfg(feature = "use-native-tls")]
use tokio_native_tls::native_tls::TlsConnector;
#[cfg(feature = "websocket")]
use {async_tungstenite::tungstenite::client::IntoClientRequest, ws_stream_tungstenite::WsStream};
#[cfg(feature = "use-rustls")]
use {
    rustls_native_certs::load_native_certs,
    tokio_rustls::rustls::{ClientConfig, RootCertStore},
};

use crate::framed::{AsyncReadWrite, Network};
#[cfg(feature = "websocket")]
use crate::websockets::{split_url, validate_response_headers};

#[derive(Debug, thiserror::Error)]
pub enum TransportError {
    #[error("Socket connect: {0}")]
    Io(#[from] std::io::Error),
    #[cfg(any(feature = "use-rustls", feature = "use-native-tls"))]
    #[error("Tls connect: {0}")]
    Tls(#[from] crate::tls::Error),
    #[cfg(feature = "proxy")]
    #[error("Proxy Connect: {0}")]
    Proxy(#[from] crate::proxy::ProxyError),
    #[cfg(feature = "websocket")]
    #[error("Invalid Url: {0}")]
    InvalidUrl(#[from] crate::websockets::UrlError),
    #[cfg(feature = "websocket")]
    #[error("Websocket response validation error: ")]
    ResponseValidation(#[from] crate::websockets::ValidationError),
    #[cfg(feature = "websocket")]
    #[error("Websocket: {0}")]
    Websocket(#[from] async_tungstenite::tungstenite::error::Error),
}

/// Transport methods. Defaults to TCP.
#[derive(Clone)]
pub enum Transport {
    Tcp,
    #[cfg(any(feature = "use-rustls", feature = "use-native-tls"))]
    Tls(TlsConfiguration),
    #[cfg(unix)]
    Unix,
    #[cfg(feature = "websocket")]
    #[cfg_attr(docsrs, doc(cfg(feature = "websocket")))]
    Ws,
    #[cfg(all(feature = "use-rustls", feature = "websocket"))]
    #[cfg_attr(docsrs, doc(cfg(all(feature = "use-rustls", feature = "websocket"))))]
    Wss(TlsConfiguration),
}

impl Default for Transport {
    fn default() -> Self {
        Self::tcp()
    }
}

impl Transport {
    /// Use regular tcp as transport (default)
    pub fn tcp() -> Self {
        Self::Tcp
    }

    #[cfg(feature = "use-rustls")]
    pub fn tls_with_default_config() -> Self {
        Self::tls_with_config(Default::default())
    }

    /// Use secure tcp with tls as transport
    #[cfg(feature = "use-rustls")]
    pub fn tls(
        ca: Vec<u8>,
        client_auth: Option<(Vec<u8>, Vec<u8>)>,
        alpn: Option<Vec<Vec<u8>>>,
    ) -> Self {
        let config = TlsConfiguration::Simple {
            ca,
            alpn,
            client_auth,
        };

        Self::tls_with_config(config)
    }

    #[cfg(any(feature = "use-rustls", feature = "use-native-tls"))]
    pub fn tls_with_config(tls_config: TlsConfiguration) -> Self {
        Self::Tls(tls_config)
    }

    #[cfg(unix)]
    pub fn unix() -> Self {
        Self::Unix
    }

    /// Use websockets as transport
    #[cfg(feature = "websocket")]
    #[cfg_attr(docsrs, doc(cfg(feature = "websocket")))]
    pub fn ws() -> Self {
        Self::Ws
    }

    /// Use secure websockets with tls as transport
    #[cfg(all(feature = "use-rustls", feature = "websocket"))]
    #[cfg_attr(docsrs, doc(cfg(all(feature = "use-rustls", feature = "websocket"))))]
    pub fn wss(
        ca: Vec<u8>,
        client_auth: Option<(Vec<u8>, Vec<u8>)>,
        alpn: Option<Vec<Vec<u8>>>,
    ) -> Self {
        let config = TlsConfiguration::Simple {
            ca,
            client_auth,
            alpn,
        };

        Self::wss_with_config(config)
    }

    #[cfg(all(feature = "use-rustls", feature = "websocket"))]
    #[cfg_attr(docsrs, doc(cfg(all(feature = "use-rustls", feature = "websocket"))))]
    pub fn wss_with_config(tls_config: TlsConfiguration) -> Self {
        Self::Wss(tls_config)
    }

    #[cfg(all(feature = "use-rustls", feature = "websocket"))]
    #[cfg_attr(docsrs, doc(cfg(all(feature = "use-rustls", feature = "websocket"))))]
    pub fn wss_with_default_config() -> Self {
        Self::Wss(Default::default())
    }
}

/// TLS configuration method
#[derive(Clone, Debug)]
#[cfg(any(feature = "use-rustls", feature = "use-native-tls"))]
pub enum TlsConfiguration {
    #[cfg(feature = "use-rustls")]
    Simple {
        /// ca certificate
        ca: Vec<u8>,
        /// alpn settings
        alpn: Option<Vec<Vec<u8>>>,
        /// tls client_authentication
        client_auth: Option<(Vec<u8>, Vec<u8>)>,
    },
    #[cfg(feature = "use-native-tls")]
    SimpleNative {
        /// ca certificate
        ca: Vec<u8>,
        /// pkcs12 binary der and
        /// password for use with der
        client_auth: Option<(Vec<u8>, String)>,
    },
    #[cfg(feature = "use-rustls")]
    /// Injected rustls ClientConfig for TLS, to allow more customisation.
    Rustls(Arc<ClientConfig>),
    #[cfg(feature = "use-native-tls")]
    /// Use default native-tls configuration
    Native,
    #[cfg(feature = "use-native-tls")]
    /// Injected native-tls TlsConnector for TLS, to allow more customisation.
    NativeConnector(TlsConnector),
}

#[cfg(feature = "use-rustls")]
impl Default for TlsConfiguration {
    fn default() -> Self {
        let mut root_cert_store = RootCertStore::empty();
        for cert in load_native_certs().expect("could not load platform certs") {
            root_cert_store.add(cert).unwrap();
        }
        let tls_config = ClientConfig::builder()
            .with_root_certificates(root_cert_store)
            .with_no_client_auth();

        Self::Rustls(Arc::new(tls_config))
    }
}

#[cfg(feature = "use-rustls")]
impl From<ClientConfig> for TlsConfiguration {
    fn from(config: ClientConfig) -> Self {
        TlsConfiguration::Rustls(Arc::new(config))
    }
}

#[cfg(feature = "use-native-tls")]
impl From<TlsConnector> for TlsConfiguration {
    fn from(connector: TlsConnector) -> Self {
        TlsConfiguration::NativeConnector(connector)
    }
}

/// Create a TCP socket, connected to the given host
pub(crate) async fn socket(
    host: String,
    network_options: &crate::options::NetworkOptions,
) -> std::io::Result<TcpStream> {
    let addrs = lookup_host(host).await?;
    let mut last_err = None;

    for addr in addrs {
        let socket = match addr {
            SocketAddr::V4(_) => TcpSocket::new_v4()?,
            SocketAddr::V6(_) => TcpSocket::new_v6()?,
        };

        socket.set_nodelay(network_options.tcp_nodelay)?;

        if let Some(send_buff_size) = network_options.tcp_send_buffer_size {
            socket.set_send_buffer_size(send_buff_size).unwrap();
        }
        if let Some(recv_buffer_size) = network_options.tcp_recv_buffer_size {
            socket.set_recv_buffer_size(recv_buffer_size).unwrap();
        }
        #[cfg(any(target_os = "android", target_os = "fuchsia", target_os = "linux"))]
        if let Some(bind_device) = &network_options.bind_device {
            // call the bind_device function only if the bind_device network option is defined
            // If binding device is None or an empty string it removes the binding,
            // which is causing PermissionDenied errors in AWS environment (lambda function).
            socket.bind_device(Some(bind_device.as_bytes()))?;
        }

        match socket.connect(addr).await {
            Ok(s) => return Ok(s),
            Err(e) => last_err = Some(e),
        };
    }

    Err(last_err.unwrap_or_else(|| std::io::Error::other("Could not resolve host to any address")))
}

/// Setup a network connection based on the provided MQTT options.
pub(crate) async fn connect<P: Protocol<Item = Packet>>(
    options: &crate::options::MqttOptions,
) -> Result<Network<P>, TransportError> {
    // Process Unix files early, as proxy is not supported for them.
    #[cfg(unix)]
    if matches!(options.transport(), Transport::Unix) {
        let socket = UnixStream::connect(Path::new(&options.broker_addr)).await?;
        let network = Network::new(
            socket,
            options.max_packet_size_in,
            options.max_packet_size_out,
        );
        return Ok(network);
    }

    // For websockets domain and port are taken directly from `broker_addr` (which is a url).
    let (domain, port) = match options.transport() {
        #[cfg(feature = "websocket")]
        Transport::Ws => split_url(&options.broker_addr)?,
        #[cfg(all(feature = "use-rustls", feature = "websocket"))]
        Transport::Wss(_) => split_url(&options.broker_addr)?,
        _ => {
            let (d, p) = options.broker_address();
            (d.to_string(), p)
        }
    };

    let tcp_stream: Box<dyn AsyncReadWrite> = {
        #[cfg(feature = "proxy")]
        let proxy = options.proxy();
        #[cfg(not(feature = "proxy"))]
        let proxy: Option<()> = None;

        match proxy {
            #[cfg(feature = "proxy")]
            Some(proxy) => {
                proxy
                    .connect(&domain, port, &options.network_options)
                    .await?
            }
            _ => {
                let addr = format!("{domain}:{port}");
                let tcp = socket(addr, &options.network_options).await?;
                Box::new(tcp)
            }
        }
    };

    let network = match options.transport() {
        Transport::Tcp => Network::new(
            tcp_stream,
            options.max_packet_size_in,
            options.max_packet_size_out,
        ),
        #[cfg(any(feature = "use-rustls", feature = "use-native-tls"))]
        Transport::Tls(tls_configuration) => {
            let socket = crate::tls::tls_connect(
                &options.broker_addr,
                options.port,
                tls_configuration,
                tcp_stream,
            )
            .await?;
            Network::new(
                socket,
                options.max_packet_size_in,
                options.max_packet_size_out,
            )
        }
        #[cfg(feature = "websocket")]
        Transport::Ws => {
            let mut request = options.broker_addr.as_str().into_client_request()?;
            request
                .headers_mut()
                .insert("Sec-WebSocket-Protocol", "mqtt".parse().unwrap());

            if let Some(request_modifier) = options.request_modifier() {
                request = request_modifier(request).await;
            }

            let (socket, response) =
                async_tungstenite::tokio::client_async(request, tcp_stream).await?;
            validate_response_headers(response)?;

            Network::new(
                WsStream::new(socket),
                options.max_packet_size_in,
                options.max_packet_size_out,
            )
        }
        #[cfg(all(feature = "use-rustls", feature = "websocket"))]
        Transport::Wss(tls_config) => {
            let mut request = options.broker_addr.as_str().into_client_request()?;
            request
                .headers_mut()
                .insert("Sec-WebSocket-Protocol", "mqtt".parse().unwrap());

            if let Some(request_modifier) = options.request_modifier() {
                request = request_modifier(request).await;
            }

            let connector = crate::tls::rustls_connector(tls_config).await?;

            let (socket, response) = async_tungstenite::tokio::client_async_tls_with_connector(
                request,
                tcp_stream,
                Some(connector),
            )
            .await?;
            validate_response_headers(response)?;

            Network::new(
                WsStream::new(socket),
                options.max_packet_size_in,
                options.max_packet_size_out,
            )
        }
        #[cfg(unix)]
        Transport::Unix => unreachable!(),
    };

    Ok(network)
}
