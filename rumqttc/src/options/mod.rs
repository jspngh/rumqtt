use std::time::Duration;
#[cfg(feature = "websocket")]
use std::{future::Future, pin::Pin, sync::Arc};

use rumqtt_bytes::{LastWill, Login, Properties};

use crate::Transport;

mod builder;
pub use builder::OptionBuilder;

#[cfg(feature = "websocket")]
type RequestModifierFn = Arc<
    dyn Fn(http::Request<()>) -> Pin<Box<dyn Future<Output = http::Request<()>> + Send>>
        + Send
        + Sync,
>;

/// Provides a way to configure low level network connection configurations
#[derive(Debug, Clone, Default)]
pub struct NetworkOptions {
    pub(crate) tcp_send_buffer_size: Option<u32>,
    pub(crate) tcp_recv_buffer_size: Option<u32>,
    pub(crate) tcp_nodelay: bool,
    pub(crate) conn_timeout: u64,
    #[cfg(any(target_os = "android", target_os = "fuchsia", target_os = "linux"))]
    pub(crate) bind_device: Option<String>,
}

/// The options to use when connecting to the MQTT broker.
#[derive(Debug, Clone)]
pub struct ConnectOptions {
    /// Client identifier
    pub(crate) client_id: String,
    /// Clean (or) persistent session
    pub(crate) clean_start: bool,
    /// Username and password to use when connecting to the broker
    pub(crate) credentials: Option<Login>,
    /// Last will that will be issued on unexpected disconnect
    pub(crate) last_will: Option<LastWill>,
    /// Properties to use when sending a connect packet
    pub(crate) properties: Properties,
}

/// Options to configure the MQTT client behaviour
///
/// Construct this using an [`OptionBuilder`].
#[derive(Clone)]
pub struct MqttOptions {
    /// Broker address that you want to connect to
    pub(crate) broker_addr: String,
    /// Broker port
    pub(crate) port: u16,
    /// What transport protocol to use
    pub(crate) transport: Transport,
    /// Keep alive time to send ping request to the broker when the connection is idle
    pub(crate) keep_alive: Duration,
    /// Maximum size of an incoming packet
    ///
    /// This is used when verifying the remaining length of a packet
    pub(crate) max_packet_size_in: u32,
    /// The maximum number of incoming inflight QoS1/QoS2 messages
    pub(crate) receive_max_in: u16,
    /// Maximum size of an outgoing packet
    ///
    /// This is checked when sending a packet to the broker.
    /// This can be overridden by the server, if it sets a lower maximum packet size.
    pub(crate) max_packet_size_out: u32,
    /// The maximum number of outgoing inflight QoS1/QoS2 messages
    ///
    /// This can be overridden by the server, if it sets a lower receive maximum.
    pub(crate) receive_max_out: u16,
    /// Minimum delay time between consecutive outgoing packets
    /// while retransmitting pending packets
    pub(crate) pending_throttle: Duration,
    /// If set to `true` MQTT acknowledgements are not sent automatically.
    /// Every incoming publish packet must be manually acknowledged with `client.ack(...)` method.
    pub(crate) manual_acks: bool,
    /// Configuration for MQTT connection
    pub(crate) connect_options: ConnectOptions,
    /// Configuration for network connection
    pub(crate) network_options: NetworkOptions,
    #[cfg(feature = "proxy")]
    /// Proxy configuration.
    pub(crate) proxy: Option<crate::Proxy>,
    #[cfg(feature = "websocket")]
    /// Modifier function for the HTTP request used to establish a websocket connection
    pub(crate) request_modifier: Option<RequestModifierFn>,
}

impl MqttOptions {
    /// Broker address
    pub fn broker_address(&self) -> (&str, u16) {
        (&self.broker_addr, self.port)
    }

    pub fn last_will(&self) -> Option<&LastWill> {
        self.connect_options.last_will.as_ref()
    }

    pub fn transport(&self) -> &Transport {
        &self.transport
    }

    /// Keep alive time
    pub fn keep_alive(&self) -> Duration {
        self.keep_alive
    }

    /// Client identifier
    pub fn client_id(&self) -> &str {
        &self.connect_options.client_id
    }

    /// Maximum packet size
    pub fn max_packet_size(&self) -> u32 {
        self.max_packet_size_in
    }

    /// Clean session
    pub fn clean_session(&self) -> bool {
        self.connect_options.clean_start
    }

    /// Security options
    pub fn credentials(&self) -> Option<&Login> {
        self.connect_options.credentials.as_ref()
    }

    /// Get connection timeout in seconds
    pub fn connection_timeout(&self) -> u64 {
        self.network_options.conn_timeout
    }

    /// Outgoing message rate
    pub fn pending_throttle(&self) -> Duration {
        self.pending_throttle
    }

    /// Number of concurrent in flight messages
    pub fn inflight(&self) -> u16 {
        self.receive_max_in
    }

    /// get manual acknowledgements
    pub fn manual_acks(&self) -> bool {
        self.manual_acks
    }

    #[cfg(feature = "proxy")]
    pub fn proxy(&self) -> Option<&crate::Proxy> {
        self.proxy.as_ref()
    }

    #[cfg(feature = "websocket")]
    pub fn request_modifier(&self) -> Option<RequestModifierFn> {
        self.request_modifier.clone()
    }
}

// Implement Debug manually because ClientConfig doesn't implement it
impl std::fmt::Debug for MqttOptions {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.debug_struct("MqttOptions")
            .field("broker_addr", &self.broker_addr)
            .field("port", &self.port)
            .field("keep_alive", &self.keep_alive)
            .field("clean_session", &self.connect_options.clean_start)
            .field("client_id", &self.connect_options.client_id)
            .field("credentials", &self.connect_options.credentials)
            .field("max_packet_size", &self.max_packet_size_in)
            .field("pending_throttle", &self.pending_throttle)
            .field("receive_max", &self.receive_max_in)
            .field("last_will", &self.connect_options.last_will)
            .field("manual_acks", &self.manual_acks)
            .finish()
    }
}
