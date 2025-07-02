use std::num::NonZero;
use std::time::Duration;

use rumqtt_bytes::{ConnectProperties, LastWill, Login};

use super::{ConnectOptions, MqttOptions, NetworkOptions};
use crate::{TlsConfiguration, Transport};

#[cfg(feature = "websocket")]
use super::RequestModifierFn;
#[cfg(feature = "proxy")]
use crate::Proxy;

/// TODO
pub struct OptionsBuilder {
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
    client_id: Option<String>,
    keep_alive: Duration,
    clean_start: bool,
    credentials: Option<Login>,
    request_channel_capacity: usize,
    max_request_batch: usize,
    max_incoming_size: usize,
    max_outgoing_size: usize,
    max_incoming_inflight: u16,
    max_outgoing_inflight: u16,
    pending_throttle: Duration,
    last_will: Option<LastWill>,
    connect_properties: Option<ConnectProperties>,
    manual_acks: bool,
    #[cfg(feature = "proxy")]
    proxy: Option<Proxy>,
    #[cfg(feature = "websocket")]
    request_modifier: Option<RequestModifierFn>,
}

impl OptionsBuilder {
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
            request_channel_capacity: 10,
            max_request_batch: 0,
            max_incoming_size: 10 * 1024,
            max_outgoing_size: 10 * 1024,
            max_incoming_inflight: 100,
            max_outgoing_inflight: 100,
            pending_throttle: Duration::from_micros(0),
            last_will: None,
            connect_properties: None,
            manual_acks: false,
            #[cfg(feature = "proxy")]
            proxy: None,
            #[cfg(feature = "websocket")]
            request_modifier: None,
        }
    }

    #[cfg(feature = "url")]
    /// Creates an [`OptionsBuilder`] object by parsing the provided string with
    /// the [url] crate's [`parse`](url::Url::parse) method.
    ///
    /// This is only available when the `url` feature is enabled.
    ///
    /// ```
    /// # use rumqttc::OptionsBuilder;
    /// let options = OptionsBuilder::from_url("mqtt://example.com:1883?client_id=123").unwrap();
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
    /// # use rumqttc::{OptionsBuilder, Transport};
    /// # use tokio_rustls::rustls::ClientConfig;
    /// # let root_cert_store = rustls::RootCertStore::empty();
    /// # let client_config = ClientConfig::builder()
    /// #    .with_root_certificates(root_cert_store)
    /// #    .with_no_client_auth();
    /// let mut options = OptionsBuilder::from_url("mqtts://example.com?client_id=123").unwrap();
    /// options.transport(Transport::tls_with_config(client_config.into()));
    /// ```
    pub fn from_url(url: impl Into<String>) -> Result<Self, OptionError> {
        let url = url::Url::parse(&url.into())?;
        Self::try_from(url)
    }

    pub fn finalize(self) -> MqttOptions {
        let client_id = self.client_id.unwrap_or_else(|| {
            // generate random client id
            todo!()
        });
        let network_options = NetworkOptions {
            tcp_send_buffer_size: self.tcp_send_buffer_size,
            tcp_recv_buffer_size: self.tcp_recv_buffer_size,
            tcp_nodelay: self.tcp_nodelay,
            conn_timeout: self.conn_timeout,
        };
        let connect_options = ConnectOptions {
            client_id,
            clean_start: self.clean_start,
            credentials: self.credentials,
            last_will: self.last_will,
            properties: self.connect_properties,
        };
        MqttOptions {
            broker_addr: self.broker_addr,
            port: self.port,
            transport: self.transport,
            keep_alive: self.keep_alive,
            request_channel_capacity: self.request_channel_capacity,
            max_request_batch: self.max_request_batch,
            max_incoming_size: self.max_incoming_size,
            max_outgoing_size: self.max_outgoing_size,
            max_incoming_inflight: self.max_incoming_inflight,
            max_outgoing_inflight: self.max_outgoing_inflight,
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
impl OptionsBuilder {
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

    /// Bind the connection to a specific network device by name
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
impl OptionsBuilder {
    /// Set the client identifier to use.
    ///
    /// If this is not set, the client will generate a random identifier.
    pub fn client_id<S: Into<String>>(mut self, client_id: S) -> Self {
        self.client_id = Some(client_id.into());
        self
    }

    /// Set the last will message
    pub fn last_will(mut self, will: LastWill) -> Self {
        self.last_will = Some(will);
        self
    }

    /// Set the username and password to use for authentication
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
        // TODO: why should we do this?
        // assert!(
        //     !self.client_id.is_empty() || clean_start,
        //     "Cannot unset clean session when client id is empty"
        // );
        self.clean_start = clean_start;
        self
    }

    /// Set the `Session Expiry Interval` connect property
    ///
    /// This specifies the duration in seconds for which the session
    /// should be maintained by the broker after the client disconnects.
    pub fn session_expiry_interval(mut self, interval: u32) -> Self {
        match self.connect_properties {
            Some(ref mut conn_props) => {
                conn_props.session_expiry_interval = Some(interval);
            }
            None => {
                self.connect_properties = Some(ConnectProperties {
                    session_expiry_interval: Some(interval),
                    ..Default::default()
                });
            }
        }
        self
    }

    /// Set the `Receive Maximum` connect property
    ///
    /// This is a limit on the number of QoS 1 and QoS 2 publications
    /// that the client is willing to process concurrently.
    pub fn receive_maximum(mut self, recv_max: u16) -> Self {
        self.max_incoming_inflight = recv_max;
        match self.connect_properties {
            Some(ref mut conn_props) => {
                conn_props.receive_maximum = Some(recv_max);
            }
            None => {
                self.connect_properties = Some(ConnectProperties {
                    receive_maximum: Some(recv_max),
                    ..Default::default()
                });
            }
        }
        self
    }

    /// Set the `Maximum Packet Size` connect property
    ///
    /// This puts a limit on the size of an incoming MQTT packet.
    pub fn max_packet_size(mut self, max_size: u32) -> Self {
        self.max_incoming_size = max_size as usize;
        match self.connect_properties {
            Some(ref mut conn_props) => {
                conn_props.max_packet_size = Some(max_size);
            }
            None => {
                self.connect_properties = Some(ConnectProperties {
                    max_packet_size: Some(max_size),
                    ..Default::default()
                });
            }
        }
        self
    }

    /// Set the `Topic Alias Maximum` connect property
    ///
    /// This is the highest value that the client will accept as a Topic Alias sent by the server.
    pub fn topic_alias_max(mut self, topic_alias_max: u16) -> Self {
        match self.connect_properties {
            Some(ref mut conn_props) => {
                conn_props.topic_alias_max = Some(topic_alias_max);
            }
            None => {
                self.connect_properties = Some(ConnectProperties {
                    topic_alias_max: Some(topic_alias_max),
                    ..Default::default()
                });
            }
        }
        self
    }

    /// Set the `Request Response Information` connect property
    ///
    /// This requests the server to return *response information* in the connack packet.
    pub fn request_response_info(mut self) -> Self {
        let request_response_info = Some(1);
        match self.connect_properties {
            Some(ref mut conn_props) => {
                conn_props.request_response_info = request_response_info;
            }
            None => {
                self.connect_properties = Some(ConnectProperties {
                    request_response_info,
                    ..Default::default()
                });
            }
        }
        self
    }

    /// Set the `Request Problem Information` connect property
    ///
    /// This indicate whether the *reason string* or *user properties* are sent in case of failures.
    /// When passing `true`, the server may include a *reason string* or *user properties* in any packet where it is allowd.
    /// When passing `false`, the server may only include a *reason string* or *user properties* in case of publish, connack or disconnect.
    pub fn request_problem_info(mut self, problem_info: bool) -> Self {
        let request_problem_info = if problem_info { Some(1) } else { Some(0) };
        match self.connect_properties {
            Some(ref mut conn_props) => {
                conn_props.request_problem_info = request_problem_info;
            }
            None => {
                self.connect_properties = Some(ConnectProperties {
                    request_problem_info,
                    ..Default::default()
                });
            }
        }
        self
    }

    /// Set user properties to be used in connect packets.
    pub fn user_properties(mut self, user_properties: Vec<(String, String)>) -> Self {
        match self.connect_properties {
            Some(ref mut conn_props) => {
                conn_props.user_properties = user_properties;
            }
            None => {
                self.connect_properties = Some(ConnectProperties {
                    user_properties,
                    ..Default::default()
                });
            }
        }
        self
    }

    /// Set the `Authentication Method` connect property
    pub fn authentication_method(mut self, method: String) -> Self {
        match self.connect_properties {
            Some(ref mut conn_props) => {
                conn_props.authentication_method = Some(method);
            }
            None => {
                self.connect_properties = Some(ConnectProperties {
                    authentication_method: Some(method),
                    ..Default::default()
                });
            }
        }
        self
    }

    /// Set the `Authentication Data` connect property
    pub fn authentication_data(mut self, data: bytes::Bytes) -> Self {
        match self.connect_properties {
            Some(ref mut conn_props) => {
                conn_props.authentication_data = Some(data);
            }
            None => {
                self.connect_properties = Some(ConnectProperties {
                    authentication_data: Some(data),
                    ..Default::default()
                });
            }
        }
        self
    }
}

// MQTT options
impl OptionsBuilder {
    /// Set the transport method to use for the connection.
    ///
    /// In most cases this should already be set by using the correct constructor method.
    pub fn transport(mut self, transport: Transport) -> Self {
        self.transport = transport;
        self
    }

    /// Set the number of seconds after which client should ping the broker when idle
    pub fn keep_alive(mut self, duration: Duration) -> Self {
        assert!(
            duration.is_zero() || duration >= Duration::from_secs(1),
            "Keep alives should be specified in seconds. Durations less than \
                a second are not allowed, except for Duration::ZERO."
        );

        self.keep_alive = duration;
        self
    }

    /// Set packet size limit for incoming packets
    pub fn max_incoming_size(mut self, incoming: usize) -> Self {
        self.max_incoming_size = incoming;
        self
    }

    /// Set packet size limit for outgoing packets
    pub fn max_outgoing_size(mut self, outgoing: usize) -> Self {
        self.max_outgoing_size = outgoing;
        self
    }

    /// Set request channel capacity
    pub fn request_channel_capacity(mut self, capacity: usize) -> Self {
        self.request_channel_capacity = capacity;
        self
    }

    /// Enables throttling and sets outoing message rate to the specified 'rate'
    pub fn pending_throttle(mut self, duration: Duration) -> Self {
        self.pending_throttle = duration;
        self
    }

    /// Set maximum number of concurrent incoming inflight messages
    pub fn incoming_inflight(mut self, inflight: NonZero<u16>) -> Self {
        self.max_incoming_inflight = inflight.get();
        self
    }

    /// Set maximum number of concurrent outgoing inflight messages
    pub fn outgoing_inflight(mut self, inflight: NonZero<u16>) -> Self {
        self.max_outgoing_inflight = inflight.get();
        self
    }

    /// set manual acknowledgements
    pub fn manual_acks(mut self, manual_acks: bool) -> Self {
        self.manual_acks = manual_acks;
        self
    }

    #[cfg(feature = "proxy")]
    pub fn proxy(mut self, proxy: Proxy) -> Self {
        self.proxy = Some(proxy);
        self
    }

    #[cfg(feature = "websocket")]
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
    #[error("Invalid request-channel-capacity value.")]
    RequestChannelCapacity,
    #[error("Invalid max-request-batch value.")]
    MaxRequestBatch,
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
impl std::convert::TryFrom<url::Url> for OptionsBuilder {
    type Error = OptionError;

    fn try_from(url: url::Url) -> Result<Self, Self::Error> {
        use std::collections::HashMap;

        let (transport, default_port) = match url.scheme() {
            "mqtt" | "tcp" => (Transport::Tcp, 1883),
            #[cfg(unix)]
            "unix" => (Transport::Unix, 0),
            #[cfg(any(feature = "use-rustls", feature = "use-native-tls"))]
            "mqtts" | "ssl" => (Transport::tls_with_default_config(), 8883),
            #[cfg(feature = "websocket")]
            "ws" => (Transport::Ws, 8000),
            #[cfg(all(feature = "use-rustls", feature = "websocket"))]
            "wss" => (Transport::wss_with_default_config(), 8000),
            _ => return Err(OptionError::Scheme),
        };

        // TODO: no host should be an error
        let host = url.host_str().unwrap_or_default().to_owned();
        let port = url.port().unwrap_or(default_port);
        let mut options = OptionsBuilder::new(transport, host, port);

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
                v.parse::<usize>()
                    .map_err(|_| OptionError::MaxIncomingPacketSize)
            })
            .transpose()?
        {
            options = options.max_incoming_size(incoming);
        }

        if let Some(outgoing) = queries
            .remove("max_outgoing_packet_size_bytes")
            .map(|v| {
                v.parse::<usize>()
                    .map_err(|_| OptionError::MaxOutgoingPacketSize)
            })
            .transpose()?
        {
            options = options.max_outgoing_size(outgoing);
        }

        if let Some(request_channel_capacity) = queries
            .remove("request_channel_capacity_num")
            .map(|v| {
                v.parse::<usize>()
                    .map_err(|_| OptionError::RequestChannelCapacity)
            })
            .transpose()?
        {
            options.request_channel_capacity = request_channel_capacity;
        }

        if let Some(max_request_batch) = queries
            .remove("max_request_batch_num")
            .map(|v| v.parse::<usize>().map_err(|_| OptionError::MaxRequestBatch))
            .transpose()?
        {
            options.max_request_batch = max_request_batch;
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
            .map(|v| v.parse::<NonZero<u16>>().map_err(|_| OptionError::Inflight))
            .transpose()?
        {
            options = options.inflight(inflight);
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
    #[should_panic] // TODO
    fn random_client_id() {
        let options = OptionsBuilder::new_tcp("localhost", 1883).finalize();
        assert!(options.client_id().starts_with("todo"));
    }

    #[test]
    #[cfg(all(feature = "use-rustls", feature = "websocket"))]
    fn no_scheme() {
        let mut mqttoptions = MqttOptions::new("client_a", "a3f8czas.iot.eu-west-1.amazonaws.com/mqtt?X-Amz-Algorithm=AWS4-HMAC-SHA256&X-Amz-Credential=MyCreds%2F20201001%2Feu-west-1%2Fiotdevicegateway%2Faws4_request&X-Amz-Date=20201001T130812Z&X-Amz-Expires=7200&X-Amz-Signature=9ae09b49896f44270f2707551581953e6cac71a4ccf34c7c3415555be751b2d1&X-Amz-SignedHeaders=host", 443);

        mqttoptions.set_transport(crate::Transport::wss(Vec::from("Test CA"), None, None));

        if let crate::Transport::Wss(TlsConfiguration::Simple {
            ca,
            client_auth,
            alpn,
        }) = mqttoptions.transport
        {
            assert_eq!(ca, Vec::from("Test CA"));
            assert_eq!(client_auth, None);
            assert_eq!(alpn, None);
        } else {
            panic!("Unexpected transport!");
        }

        assert_eq!(mqttoptions.broker_addr, "a3f8czas.iot.eu-west-1.amazonaws.com/mqtt?X-Amz-Algorithm=AWS4-HMAC-SHA256&X-Amz-Credential=MyCreds%2F20201001%2Feu-west-1%2Fiotdevicegateway%2Faws4_request&X-Amz-Date=20201001T130812Z&X-Amz-Expires=7200&X-Amz-Signature=9ae09b49896f44270f2707551581953e6cac71a4ccf34c7c3415555be751b2d1&X-Amz-SignedHeaders=host");
    }

    #[test]
    #[cfg(feature = "url")]
    fn from_url() {
        fn opt(s: &str) -> Result<MqttOptions, OptionError> {
            MqttOptions::parse_url(s)
        }
        fn ok(s: &str) -> MqttOptions {
            opt(s).expect("valid options")
        }
        fn err(s: &str) -> OptionError {
            opt(s).expect_err("invalid options")
        }

        let v = ok("mqtt://host:42?client_id=foo");
        assert_eq!(v.broker_address(), ("host".to_owned(), 42));
        assert_eq!(v.client_id(), "foo".to_owned());

        let v = ok("mqtt://host:42?client_id=foo&keep_alive_secs=5");
        assert_eq!(v.keep_alive, Duration::from_secs(5));

        assert_eq!(err("mqtt://host:42"), OptionError::ClientId);
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
            err("mqtt://host:42?client_id=foo&request_channel_capacity_num=foo"),
            OptionError::RequestChannelCapacity
        );
        assert_eq!(
            err("mqtt://host:42?client_id=foo&max_request_batch_num=foo"),
            OptionError::MaxRequestBatch
        );
        assert_eq!(
            err("mqtt://host:42?client_id=foo&pending_throttle_usecs=foo"),
            OptionError::PendingThrottle
        );
        assert_eq!(
            err("mqtt://host:42?client_id=foo&inflight_num=foo"),
            OptionError::Inflight
        );
    }
}
