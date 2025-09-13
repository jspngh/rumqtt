//! A pure rust MQTT client which strives to be robust, efficient and easy to use.
//! This library is backed by an async (tokio) eventloop which handles all the
//! robustness and and efficiency parts of MQTT but naturally fits into both sync
//! and async worlds as we'll see
//!
//! Let's jump into examples right away
//!
//! A simple synchronous publish and subscribe
//! ----------------------------
//!
//! ```no_run
//! use rumqttc::{OptionBuilder, Client, QoS};
//! use std::time::Duration;
//! use std::thread;
//!
//! let options = OptionBuilder::new_tcp("test.mosquitto.org", 1883)
//!     .client_id("rumqtt-sync")
//!     .keep_alive(Duration::from_secs(5))
//!     .finalize();
//!
//! let (mut client, mut connection) = Client::new(options, 10);
//! client.subscribe("hello/rumqtt", QoS::AtMostOnce).unwrap();
//! thread::spawn(move || for i in 0..10 {
//!    client.publish("hello/rumqtt", QoS::AtLeastOnce, false, vec![i; i as usize]).unwrap();
//!    thread::sleep(Duration::from_millis(100));
//! });
//!
//! // Iterate to poll the eventloop for connection progress
//! for (i, notification) in connection.iter().enumerate() {
//!     println!("Notification = {:?}", notification);
//! }
//! ```
//!
//! A simple asynchronous publish and subscribe
//! ------------------------------
//!
//! ```no_run
//! use rumqttc::{OptionBuilder, AsyncClient, QoS};
//! use tokio::{task, time};
//! use std::time::Duration;
//! use std::error::Error;
//!
//! # #[tokio::main(flavor = "current_thread")]
//! # async fn main() {
//! let options = OptionBuilder::new_tcp("test.mosquitto.org", 1883)
//!     .client_id("rumqtt-async")
//!     .keep_alive(Duration::from_secs(5))
//!     .finalize();
//!
//! let (mut client, mut eventloop) = AsyncClient::new(options, 10);
//! client.subscribe("hello/rumqtt", QoS::AtMostOnce).await.unwrap();
//!
//! task::spawn(async move {
//!     for i in 0..10 {
//!         client.publish("hello/rumqtt", QoS::AtLeastOnce, false, vec![i; i as usize]).await.unwrap();
//!         time::sleep(Duration::from_millis(100)).await;
//!     }
//! });
//!
//! loop {
//!     let notification = eventloop.poll().await.unwrap();
//!     println!("Received = {:?}", notification);
//! }
//! # }
//! ```
//!
//! Quick overview of features
//! - Eventloop orchestrates outgoing/incoming packets concurrently and handles the state
//! - Pings the broker when necessary and detects client side half open connections as well
//! - Throttling of outgoing packets (todo)
//! - Queue size based flow control on outgoing packets
//! - Automatic reconnections by just continuing the `eventloop.poll()`/`connection.iter()` loop
//! - Natural backpressure to client APIs during bad network
//!
//! In short, everything necessary to maintain a robust connection
//!
//! Since the eventloop is externally polled (with `iter()/poll()` in a loop)
//! out side the library and `Eventloop` is accessible, users can
//! - Distribute incoming messages based on topics
//! - Stop it when required
//! - Access internal state for use cases like graceful shutdown or to modify options before reconnection
//!
//! ## Important notes
//!
//! - Looping on `connection.iter()`/`eventloop.poll()` is necessary to run the
//!   event loop and make progress. It yields incoming and outgoing activity
//!   notifications which allows customization as you see fit.
//!
//! - Blocking inside the `connection.iter()`/`eventloop.poll()` loop will block
//!   connection progress.
//!
//! ## FAQ
//! **Connecting to a broker using raw ip doesn't work**
//!
//! You cannot create a TLS connection to a bare IP address with a self-signed
//! certificate. This is a [limitation of rustls](https://github.com/ctz/rustls/issues/184).
//! One workaround, which only works under *nix/BSD-like systems, is to add an
//! entry to wherever your DNS resolver looks (e.g. `/etc/hosts`) for the bare IP
//! address and use that name in your code.
#![cfg_attr(docsrs, feature(doc_cfg))]

mod client;
mod eventloop;
mod framed;
mod options;
mod state;
mod topic;
mod transport;

#[cfg(feature = "proxy")]
mod proxy;
#[cfg(any(feature = "use-rustls", feature = "use-native-tls"))]
mod tls;
#[cfg(feature = "websocket")]
mod websockets;

pub use rumqtt_bytes::{Packet, QoS};

pub use client::{
    AsyncClient, Client, ClientError, Connection, Iter, RecvError, RecvTimeoutError, TryRecvError,
};
pub use eventloop::{ConnectionError, Event, EventLoop};
pub use options::*;
pub use state::{MqttState, StateError};
pub use transport::*;

#[cfg(any(feature = "use-rustls", feature = "use-native-tls"))]
pub use tls::Error as TlsError;
#[cfg(feature = "use-native-tls")]
pub use tokio_native_tls;
#[cfg(feature = "use-rustls")]
pub use tokio_rustls;

#[cfg(feature = "proxy")]
pub use proxy::{Proxy, ProxyAuth, ProxyType};

/// Types that use an MQTT 5.0 implementation
pub mod v5 {
    /// [EventLoop](super::EventLoop) for MQTT 5.0
    pub type EventLoop = super::EventLoop<rumqtt_bytes::V5>;
    /// [AsyncClient](super::AsyncClient) for MQTT 5.0
    pub type AsyncClient = super::AsyncClient<rumqtt_bytes::V5>;
    /// Synchronous [Client](super::Client) for MQTT 5.0
    pub type Client = super::Client<rumqtt_bytes::V5>;
    /// [Connection](super::Connection) for MQTT 5.0
    pub type Connection = super::Connection<rumqtt_bytes::V5>;
}

pub type Incoming = Packet;

/// Current outgoing activity on the eventloop
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outgoing {
    /// Publish packet with packet identifier. 0 implies QoS 0
    Publish(u16),
    /// Subscribe packet with packet identifier
    Subscribe(u16),
    /// Unsubscribe packet with packet identifier
    Unsubscribe(u16),
    /// PubAck packet
    PubAck(u16),
    /// PubRec packet
    PubRec(u16),
    /// PubRel packet
    PubRel(u16),
    /// PubComp packet
    PubComp(u16),
    /// Ping request packet
    PingReq,
    /// Ping response packet
    PingResp,
    /// Disconnect packet
    Disconnect,
    /// Await for an ack for more outgoing progress
    AwaitAck(u16),
}
