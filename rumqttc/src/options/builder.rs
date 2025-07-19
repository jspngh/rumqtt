use std::time::Duration;

use bytes::Bytes;
use rumqtt_bytes::{LastWill, Login, Properties, Property};

use super::{ConnectOptions, MqttOptions, NetworkOptions};
use crate::{TlsConfiguration, Transport};

#[cfg(feature = "websocket")]
use super::RequestModifierFn;
#[cfg(feature = "proxy")]
use crate::Proxy;

/// Create [`MqttOptions`](super::MqttOptions) using a builder pattern.
pub struct OptionBuilder {
    // network options
    tcp_send_buffer_size: Option<u32>,
    tcp_recv_buffer_size: Option<u32>,
    tcp_nodelay: bool,
    conn_timeout: u64,
    #[cfg(any(target_os = "android", target_os = "fuchsia", target_os = "linux"))]
    bind_device: Option<String>,
    // mqtt options
    transport: Transport,
    broker_addr: String,
    port: u16,

    // TODO: split this off into separate builder
    client_id: Option<String>,
    keep_alive: Duration,
    clean_start: bool,
    credentials: Option<Login>,
    last_will: Option<LastWill>,
    session_expiry_interval: Option<u32>,
    topic_alias_maximum: Option<u16>,
    request_response_information: Option<bool>,
    request_problem_information: Option<bool>,
    user_properties: Vec<(String, String)>,
    authentication_method: Option<String>,
    authentication_data: Option<Bytes>,

    max_packet_size_in: u32,
    max_packet_size_out: u32,
    receive_max_in: u16,
    receive_max_out: u16,
    pending_throttle: Duration,
    manual_acks: bool,
    #[cfg(feature = "proxy")]
    proxy: Option<Proxy>,
    #[cfg(feature = "websocket")]
    request_modifier: Option<RequestModifierFn>,
}

impl OptionBuilder {
    /// Create a new `OptionBuilder` for TCP connections
    pub fn new_tcp(host: impl Into<String>, port: u16) -> Self {
        Self::new(Transport::Tcp, host.into(), port)
    }

    /// Create a new `OptionBuilder` for TLS connections
    #[cfg(any(feature = "use-rustls", feature = "use-native-tls"))]
    pub fn new_tls(host: impl Into<String>, port: u16, config: TlsConfiguration) -> Self {
        Self::new(Transport::Tls(config), host.into(), port)
    }

    /// Create a new `OptionBuilder` for Unix domain socket connections
    #[cfg(unix)]
    pub fn new_unix(path: impl Into<String>) -> Self {
        Self::new(Transport::Unix, path.into(), 0)
    }

    /// Create a new `OptionBuilder` for websocket connections
    #[cfg(feature = "websocket")]
    pub fn new_ws(host: impl Into<String>, port: u16) -> Self {
        Self::new(Transport::Ws, host.into(), port)
    }

    /// Create a new `OptionBuilder` for secure websocket connections
    #[cfg(all(feature = "websocket", feature = "use-rustls"))]
    pub fn new_wss(host: impl Into<String>, port: u16, config: TlsConfiguration) -> Self {
        Self::new(Transport::Wss(config), host.into(), port)
    }

    fn new(transport: Transport, broker_addr: String, port: u16) -> Self {
        Self {
            // default network options
            tcp_send_buffer_size: None,
            tcp_recv_buffer_size: None,
            tcp_nodelay: false,
            conn_timeout: 5,
            #[cfg(any(target_os = "android", target_os = "fuchsia", target_os = "linux"))]
            bind_device: None,
            // default mqtt options
            transport,
            broker_addr,
            port,
            client_id: None,
            keep_alive: Duration::from_secs(60),
            clean_start: true,
            credentials: None,
            last_will: None,
            session_expiry_interval: None,
            topic_alias_maximum: None,
            request_response_information: None,
            request_problem_information: None,
            user_properties: Vec::new(),
            authentication_method: None,
            authentication_data: None,
            max_packet_size_in: 10 * 1024,
            max_packet_size_out: 10 * 1024,
            receive_max_in: 100,
            receive_max_out: 100,
            pending_throttle: Duration::from_micros(0),
            manual_acks: false,
            #[cfg(feature = "proxy")]
            proxy: None,
            #[cfg(feature = "websocket")]
            request_modifier: None,
        }
    }

    #[cfg(feature = "url")]
    /// Creates an [`OptionBuilder`] object by parsing the provided string with
    /// the [url] crate's [`parse`](url::Url::parse) method.
    ///
    /// This is only available when the `url` feature is enabled.
    ///
    /// ```
    /// # use rumqttc::OptionBuilder;
    /// let options = OptionBuilder::from_url("mqtt://example.com:1883?client_id=123").unwrap();
    /// ```
    ///
    /// **NOTE:** A url must be prefixed with one of either `tcp://`, `mqtt://`, `unix://`,
    /// `ssl://`, `mqtts://`, `ws://` or `wss://` to denote the protocol
    /// for establishing a connection with the broker.
    ///
    /// **NOTE:** Encrypted connections(i.e. `mqtts://`, `ssl://`, `wss://`) by default use the
    /// system's root certificates.
    /// For custom configuration, the [`set_transport`](Self::set_transport) method can be used.
    ///
    /// ```ignore
    /// # use rumqttc::{OptionBuilder, Transport};
    /// # use tokio_rustls::rustls::ClientConfig;
    /// # let root_cert_store = rustls::RootCertStore::empty();
    /// # let client_config = ClientConfig::builder()
    /// #    .with_root_certificates(root_cert_store)
    /// #    .with_no_client_auth();
    /// let mut options = OptionBuilder::from_url("mqtts://example.com?client_id=123").unwrap();
    /// options.transport(Transport::tls_with_config(client_config.into()));
    /// ```
    pub fn from_url(url: impl Into<String>) -> Result<Self, OptionError> {
        let url = url::Url::parse(&url.into())?;
        Self::try_from(url)
    }

    pub fn finalize(self) -> MqttOptions {
        let client_id = self.client_id.unwrap_or_default();
        if client_id.is_empty() && !self.clean_start {
            // We do not panic or return an error,
            // but at least warn the user of this misconfiguration.
            log::warn!("An empty client id without a clean session will be rejected.");
        }

        let network_options = NetworkOptions {
            tcp_send_buffer_size: self.tcp_send_buffer_size,
            tcp_recv_buffer_size: self.tcp_recv_buffer_size,
            tcp_nodelay: self.tcp_nodelay,
            conn_timeout: self.conn_timeout,
            #[cfg(any(target_os = "android", target_os = "fuchsia", target_os = "linux"))]
            bind_device: self.bind_device,
        };

        let mut connect_properties = Properties::new();
        connect_properties.add(Property::ReceiveMaximum(self.receive_max_in));
        connect_properties.add(Property::MaximumPacketSize(self.max_packet_size_in));

        if let Some(interval) = self.session_expiry_interval {
            connect_properties.add(Property::SessionExpiryInterval(interval));
        }
        if let Some(x) = self.topic_alias_maximum {
            connect_properties.add(Property::TopicAliasMaximum(x));
        }
        if let Some(x) = self.request_response_information {
            connect_properties.add(Property::RequestResponseInformation(x));
        }
        if let Some(x) = self.request_problem_information {
            connect_properties.add(Property::RequestProblemInformation(x));
        }
        for (name, value) in self.user_properties {
            connect_properties.add(Property::UserProperty { name, value });
        }
        if let Some(x) = self.authentication_method {
            connect_properties.add(Property::AuthenticationMethod(x));
        }
        if let Some(x) = self.authentication_data {
            connect_properties.add(Property::AuthenticationData(x));
        }

        let connect_options = ConnectOptions {
            client_id,
            clean_start: self.clean_start,
            credentials: self.credentials,
            last_will: self.last_will,
            properties: connect_properties,
        };

        MqttOptions {
            broker_addr: self.broker_addr,
            port: self.port,
            transport: self.transport,
            keep_alive: self.keep_alive,
            max_packet_size_in: self.max_packet_size_in,
            max_packet_size_out: self.max_packet_size_out,
            receive_max_in: self.receive_max_in,
            receive_max_out: self.receive_max_out,
            pending_throttle: self.pending_throttle,
            manual_acks: self.manual_acks,
            connect_options,
            network_options,
            #[cfg(feature = "proxy")]
            proxy: self.proxy,
            #[cfg(feature = "websocket")]
            request_modifier: self.request_modifier,
        }
    }
}

// Network options
impl OptionBuilder {
    pub fn tcp_nodelay(mut self, nodelay: bool) -> Self {
        self.tcp_nodelay = nodelay;
        self
    }

    pub fn tcp_send_buffer_size(mut self, size: u32) -> Self {
        self.tcp_send_buffer_size = Some(size);
        self
    }

    pub fn tcp_recv_buffer_size(mut self, size: u32) -> Self {
        self.tcp_recv_buffer_size = Some(size);
        self
    }

    /// Set the connection timeout in seconds
    pub fn connection_timeout(mut self, timeout: u64) -> Self {
        self.conn_timeout = timeout;
        self
    }

    /// Bind the connection to a specific network device by name.
    #[cfg(any(target_os = "android", target_os = "fuchsia", target_os = "linux"))]
    #[cfg_attr(
        docsrs,
        doc(cfg(any(target_os = "android", target_os = "fuchsia", target_os = "linux")))
    )]
    pub fn bind_device(mut self, bind_device: &str) -> Self {
        self.bind_device = Some(bind_device.to_string());
        self
    }
}

// Connect options
impl OptionBuilder {
    /// Set the client identifier to use.
    ///
    /// A broker must support client identifiers of at least 23 bytes,
    /// using alphanumeric characters, but may support more.
    ///
    /// If this is not set, an empty client id will be used.
    /// A broker that supports this will generate a random identifier.
    ///
    /// This *must* be set when [clean_start](Self::clean_start) is `false`.
    pub fn client_id<S: Into<String>>(mut self, client_id: S) -> Self {
        self.client_id = Some(client_id.into());
        self
    }

    /// Set the last will message.
    pub fn last_will(mut self, will: LastWill) -> Self {
        self.last_will = Some(will);
        self
    }

    /// Set the username and password to use for authentication.
    pub fn credentials<U, P>(mut self, username: U, password: P) -> Self
    where
        U: Into<String>,
        P: Into<String>,
    {
        self.credentials = Some(Login::new(username, password));
        self
    }

    /// `clean_start = true` removes all the state from queues & instructs the broker
    /// to clean all the client state when client disconnects.
    ///
    /// When set `false`, broker will hold the client state and performs pending
    /// operations on the client when reconnection with same `client_id`
    /// happens. Local queue state is also held to retransmit packets after reconnection.
    pub fn clean_start(mut self, clean_start: bool) -> Self {
        self.clean_start = clean_start;
        self
    }

    /// Set the `Session Expiry Interval` connect property.
    ///
    /// This specifies the duration in seconds for which the session
    /// should be maintained by the broker after the client disconnects.
    pub fn session_expiry_interval(mut self, interval: u32) -> Self {
        self.session_expiry_interval = Some(interval);
        self
    }

    /// Set the `Receive Maximum` connect property.
    ///
    /// This is a limit on the number of QoS 1 and QoS 2 publications
    /// that the client is willing to process concurrently.
    pub fn receive_maximum(mut self, recv_max: u16) -> Self {
        self.receive_max_in = recv_max;
        self
    }

    /// Set the `Maximum Packet Size` connect property.
    ///
    /// This puts a limit on the size of an incoming MQTT packet.
    /// This value will be capped to 256MB, which is the maximum allowed by the protocol.
    pub fn max_packet_size(mut self, max_size: u32) -> Self {
        self.max_packet_size_in = std::cmp::min(max_size, 256 * 1024 * 1024);
        self
    }

    /// Set the `Topic Alias Maximum` connect property.
    ///
    /// This is the highest value that the client will accept as a Topic Alias sent by the server.
    pub fn topic_alias_max(mut self, topic_alias_max: u16) -> Self {
        self.topic_alias_maximum = Some(topic_alias_max);
        self
    }

    /// Set the `Request Response Information` connect property.
    ///
    /// This requests the server to return *response information* in the connack packet.
    pub fn request_response_info(mut self) -> Self {
        self.request_problem_information = Some(true);
        self
    }

    /// Set the `Request Problem Information` connect property.
    ///
    /// This indicate whether the *reason string* or *user properties* are sent in case of failures.
    /// When passing `true`, the server may include a *reason string* or *user properties*
    /// in any packet where it is allowed.
    /// When passing `false`, the server may only include a *reason string* or *user properties*
    /// in case of publish, connack or disconnect.
    pub fn request_problem_info(mut self, problem_info: bool) -> Self {
        self.request_problem_information = Some(problem_info);
        self
    }

    /// Set user properties to be used in connect packets.
    pub fn user_properties(mut self, user_properties: Vec<(String, String)>) -> Self {
        self.user_properties = user_properties;
        self
    }

    /// Set the `Authentication Method` connect property.
    pub fn authentication_method(mut self, method: String) -> Self {
        self.authentication_method = Some(method);
        self
    }

    /// Set the `Authentication Data` connect property.
    pub fn authentication_data(mut self, data: bytes::Bytes) -> Self {
        self.authentication_data = Some(data);
        self
    }
}

// MQTT options
impl OptionBuilder {
    /// Set the transport method to use for the connection.
    ///
    /// In most cases this should already be set by using the correct constructor method.
    pub fn transport(mut self, transport: Transport) -> Self {
        self.transport = transport;
        self
    }

    /// Set the maximum time interval between message.
    ///
    /// If no other messages are being sent, ping requests are used to keep the connection alive.
    /// Setting this to zero disables this functionality.
    ///
    /// In MQTT 5.0, the server may override this value.
    pub fn keep_alive(mut self, duration: Duration) -> Self {
        assert!(
            duration.is_zero() || duration >= Duration::from_secs(1),
            "Keep alives should be specified in seconds. Durations less than \
                a second are not allowed, except for Duration::ZERO."
        );

        self.keep_alive = duration;
        self
    }

    /// Set a limit on the size of outgoing packets.
    ///
    /// In MQTT 5.0, the server may override this if it sends a smaller 'Maximum Packet Size'.
    pub fn max_outgoing_size(mut self, outgoing: u32) -> Self {
        self.max_packet_size_out = outgoing;
        self
    }

    /// Set the maximum number of outgoing inflight QoS1/QoS2 messages.
    ///
    /// Space will be pre-allocated for this amount of packets,
    /// so it is important to select an appropriate value.
    /// In MQTT 5.0, the server may override this if it sends a smaller 'Receive Maximum'.
    pub fn outgoing_inflight(mut self, inflight: u16) -> Self {
        self.receive_max_out = inflight;
        self
    }

    /// Enables throttling for pending messages.
    ///
    /// The specified duration will be used as the time between sending pending packets.
    pub fn pending_throttle(mut self, duration: Duration) -> Self {
        self.pending_throttle = duration;
        self
    }

    /// Enable manual acknowledgements.
    ///
    /// When this is active, the client has to manually send acknowledgement
    /// messages for incoming publish packets.
    pub fn manual_acks(mut self, manual_acks: bool) -> Self {
        self.manual_acks = manual_acks;
        self
    }

    #[cfg(feature = "proxy")]
    /// Set a proxy configuration.
    pub fn proxy(mut self, proxy: Proxy) -> Self {
        self.proxy = Some(proxy);
        self
    }

    #[cfg(feature = "websocket")]
    /// Set a modifier function for the HTTP request used in WebSocket connections.
    ///
    /// This should be an asynchronous function that takes an `http::Request<()>`
    /// and returns the modified request.
    pub fn request_modifier<F, O>(mut self, request_modifier: F) -> Self
    where
        F: Fn(http::Request<()>) -> O + Send + Sync + 'static,
        O: std::future::IntoFuture<Output = http::Request<()>> + 'static,
        O::IntoFuture: Send,
    {
        self.request_modifier = Some(std::sync::Arc::new(move |request| {
            let request_modifier = request_modifier(request).into_future();
            Box::pin(request_modifier)
        }));

        self
    }
}

#[cfg(feature = "url")]
#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum OptionError {
    #[error("Unsupported URL scheme.")]
    Scheme,
    #[error("Invalid keep-alive value.")]
    KeepAlive,
    #[error("Invalid clean-session value.")]
    CleanSession,
    #[error("Invalid max-incoming-packet-size value.")]
    MaxIncomingPacketSize,
    #[error("Invalid max-outgoing-packet-size value.")]
    MaxOutgoingPacketSize,
    #[error("Invalid pending-throttle value.")]
    PendingThrottle,
    #[error("Invalid inflight value.")]
    Inflight,
    #[error("Unknown option: {0}")]
    Unknown(String),
    #[error("Couldn't parse option from url: {0}")]
    Parse(#[from] url::ParseError),
}

#[cfg(feature = "url")]
impl std::convert::TryFrom<url::Url> for OptionBuilder {
    type Error = OptionError;

    fn try_from(url: url::Url) -> Result<Self, Self::Error> {
        use std::collections::HashMap;

        let host = url
            .host_str()
            .ok_or(OptionError::Parse(url::ParseError::EmptyHost));
        let mut options = match url.scheme() {
            "mqtt" | "tcp" => OptionBuilder::new_tcp(host?, url.port().unwrap_or(1883)),
            #[cfg(any(feature = "use-rustls", feature = "use-native-tls"))]
            "mqtts" | "ssl" => OptionBuilder::new_tls(
                host?,
                url.port().unwrap_or(8883),
                TlsConfiguration::default(),
            ),
            #[cfg(feature = "websocket")]
            "ws" => OptionBuilder::new_ws(host?, url.port().unwrap_or(8000)),
            #[cfg(all(feature = "use-rustls", feature = "websocket"))]
            "wss" => OptionBuilder::new_wss(
                host?,
                url.port().unwrap_or(8000),
                TlsConfiguration::default(),
            ),
            #[cfg(unix)]
            "unix" => OptionBuilder::new_unix(url.path()),
            _ => return Err(OptionError::Scheme),
        };

        let mut queries = url.query_pairs().collect::<HashMap<_, _>>();

        if let Some(id) = queries.remove("client_id") {
            options = options.client_id(id);
        }

        if let Some(keep_alive) = queries
            .remove("keep_alive_secs")
            .map(|v| v.parse::<u64>().map_err(|_| OptionError::KeepAlive))
            .transpose()?
        {
            options = options.keep_alive(Duration::from_secs(keep_alive));
        }

        if let Some(clean_session) = queries
            .remove("clean_session") // TODO: should this be renamed to `clean_start`?
            .map(|v| v.parse::<bool>().map_err(|_| OptionError::CleanSession))
            .transpose()?
        {
            options = options.clean_start(clean_session);
        }

        if let Some((username, password)) = {
            match url.username() {
                "" => None,
                username => Some((
                    username.to_owned(),
                    url.password().unwrap_or_default().to_owned(),
                )),
            }
        } {
            options = options.credentials(username, password);
        }

        if let Some(incoming) = queries
            .remove("max_incoming_packet_size_bytes")
            .map(|v| {
                v.parse::<u32>()
                    .map_err(|_| OptionError::MaxIncomingPacketSize)
            })
            .transpose()?
        {
            options = options.max_packet_size(incoming);
        }

        if let Some(outgoing) = queries
            .remove("max_outgoing_packet_size_bytes")
            .map(|v| {
                v.parse::<u32>()
                    .map_err(|_| OptionError::MaxOutgoingPacketSize)
            })
            .transpose()?
        {
            options = options.max_outgoing_size(outgoing);
        }

        if let Some(pending_throttle) = queries
            .remove("pending_throttle_usecs")
            .map(|v| v.parse::<u64>().map_err(|_| OptionError::PendingThrottle))
            .transpose()?
        {
            options = options.pending_throttle(Duration::from_micros(pending_throttle));
        }

        if let Some(inflight) = queries
            .remove("inflight_num")
            .map(|v| v.parse::<u16>().map_err(|_| OptionError::Inflight))
            .transpose()?
        {
            options = options.receive_maximum(inflight);
        }

        if let Some((opt, _)) = queries.into_iter().next() {
            return Err(OptionError::Unknown(opt.into_owned()));
        }

        Ok(options)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn default_client_id() {
        let options = OptionBuilder::new_tcp("localhost", 1883).finalize();
        assert!(options.client_id().is_empty());
    }

    #[test]
    #[cfg(all(feature = "use-rustls", feature = "websocket"))]
    fn no_scheme() {
        let options = OptionBuilder::new_wss(
            "a3f8czas.iot.eu-west-1.amazonaws.com/mqtt",
            443,
            TlsConfiguration::Simple {
                ca: Vec::from("Test CA"),
                client_auth: None,
                alpn: None,
            },
        )
        .finalize();

        if let crate::Transport::Wss(TlsConfiguration::Simple {
            ca,
            client_auth,
            alpn,
        }) = options.transport
        {
            assert_eq!(ca, Vec::from("Test CA"));
            assert_eq!(client_auth, None);
            assert_eq!(alpn, None);
        } else {
            panic!("Unexpected transport!");
        }

        assert_eq!(
            options.broker_addr,
            "a3f8czas.iot.eu-west-1.amazonaws.com/mqtt"
        );
    }

    #[test]
    #[cfg(feature = "url")]
    fn from_url() {
        fn opt(s: &str) -> Result<MqttOptions, OptionError> {
            OptionBuilder::from_url(s).map(|o| o.finalize())
        }
        fn ok(s: &str) -> MqttOptions {
            opt(s).expect("valid options")
        }
        fn err(s: &str) -> OptionError {
            opt(s).expect_err("invalid options")
        }

        let v = ok("mqtt://host:42?client_id=foo");
        assert_eq!(v.broker_address(), ("host", 42));
        assert_eq!(v.client_id(), "foo");

        let v = ok("mqtt://host:42?client_id=foo&keep_alive_secs=5");
        assert_eq!(v.keep_alive, Duration::from_secs(5));

        assert_eq!(
            err("mqtt://host:42?client_id=foo&foo=bar"),
            OptionError::Unknown("foo".to_owned())
        );
        assert_eq!(err("mqt://host:42?client_id=foo"), OptionError::Scheme);
        assert_eq!(
            err("mqtt://host:42?client_id=foo&keep_alive_secs=foo"),
            OptionError::KeepAlive
        );
        assert_eq!(
            err("mqtt://host:42?client_id=foo&clean_session=foo"),
            OptionError::CleanSession
        );
        assert_eq!(
            err("mqtt://host:42?client_id=foo&max_incoming_packet_size_bytes=foo"),
            OptionError::MaxIncomingPacketSize
        );
        assert_eq!(
            err("mqtt://host:42?client_id=foo&max_outgoing_packet_size_bytes=foo"),
            OptionError::MaxOutgoingPacketSize
        );
        assert_eq!(
            err("mqtt://host:42?client_id=foo&pending_throttle_usecs=foo"),
            OptionError::PendingThrottle
        );
        assert_eq!(
            err("mqtt://host:42?client_id=foo&inflight_num=foo"),
            OptionError::Inflight
        );

        let v = ok("unix:/tmp/mqtt.sock");
        assert_eq!(v.broker_address(), ("/tmp/mqtt.sock", 0));

        let v = ok("unix:///tmp/mqtt.sock?client_id=foo");
        assert_eq!(v.broker_address(), ("/tmp/mqtt.sock", 0));
        assert_eq!(v.client_id(), "foo");
    }
}
