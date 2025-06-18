use std::fmt::{self, Debug, Formatter};
use std::time::Duration;

#[cfg(feature = "websocket")]
use std::future::Future;

use rumqtt_bytes::{ConnectProperties, LastWill, Login};

use crate::Transport;

#[cfg(feature = "proxy")]
use crate::Proxy;

mod builder;
pub use builder::OptionsBuilder;

#[cfg(feature = "websocket")]
type RequestModifierFn = std::sync::Arc<
    dyn Fn(http::Request<()>) -> std::pin::Pin<Box<dyn Future<Output = http::Request<()>> + Send>>
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
    pub(crate) properties: Option<ConnectProperties>,
}

/// Options to configure the MQTT client behaviour
///
/// Construct this using an [`OptionsBuilder`].
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
    /// Request (publish, subscribe) channel capacity
    pub(crate) request_channel_capacity: usize,
    /// Max internal request batching
    pub(crate) max_request_batch: usize,
    /// Maximum size of an incoming packet size
    ///
    /// This is used when verifying the remaining length of a packet
    pub(crate) max_incoming_size: usize,
    /// Maximum size of an outgoing packet size
    ///
    /// This is used when checking a publish packet size.
    /// This can be overridden by the server, if it sets a lower maximum.
    pub(crate) max_outgoing_size: usize,
    /// Maximum number of incoming inflight messages
    pub(crate) max_incoming_inflight: u16,
    /// Maximum number of outgoing inflight messages
    ///
    /// This can be overridden by the server, if it sets a lower maximum.
    pub(crate) max_outgoing_inflight: u16,
    /// Minimum delay time between consecutive outgoing packets
    /// while retransmitting pending packets
    pub(crate) pending_throttle: Duration,
    /// If set to `true` MQTT acknowledgements are not sent automatically.
    /// Every incoming publish packet must be manually acknowledged with `client.ack(...)` method.
    pub(crate) manual_acks: bool,
    // TODO
    pub(crate) connect_options: ConnectOptions,
    // TODO
    pub(crate) network_options: NetworkOptions,
    #[cfg(feature = "proxy")]
    /// Proxy configuration.
    pub(crate) proxy: Option<Proxy>,
    #[cfg(feature = "websocket")]
    // TODO
    pub(crate) request_modifier: Option<RequestModifierFn>,
}

impl MqttOptions {
    /// Broker address
    pub fn broker_address(&self) -> (String, u16) {
        (self.broker_addr.clone(), self.port)
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
    pub fn max_packet_size(&self) -> usize {
        self.max_incoming_size
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

    /// Request channel capacity
    pub fn request_channel_capacity(&self) -> usize {
        self.request_channel_capacity
    }

    /// Outgoing message rate
    pub fn pending_throttle(&self) -> Duration {
        self.pending_throttle
    }

    /// Number of concurrent in flight messages
    pub fn inflight(&self) -> u16 {
        self.max_incoming_inflight
    }

    /// get manual acknowledgements
    pub fn manual_acks(&self) -> bool {
        self.manual_acks
    }

    #[cfg(feature = "proxy")]
    pub fn proxy(&self) -> Option<Proxy> {
        self.proxy.clone()
    }

    #[cfg(feature = "websocket")]
    pub fn request_modifier(&self) -> Option<RequestModifierFn> {
        self.request_modifier.clone()
    }
}

// Implement Debug manually because ClientConfig doesn't implement it
impl Debug for MqttOptions {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.debug_struct("MqttOptions")
            .field("broker_addr", &self.broker_addr)
            .field("port", &self.port)
            .field("keep_alive", &self.keep_alive)
            .field("clean_session", &self.connect_options.clean_start)
            .field("client_id", &self.connect_options.client_id)
            .field("credentials", &self.connect_options.credentials)
            .field("max_packet_size", &self.max_incoming_size)
            .field("request_channel_capacity", &self.request_channel_capacity)
            .field("max_request_batch", &self.max_request_batch)
            .field("pending_throttle", &self.pending_throttle)
            .field("inflight", &self.max_incoming_inflight)
            .field("last_will", &self.connect_options.last_will)
            .field("manual_acks", &self.manual_acks)
            .finish()
    }
}
