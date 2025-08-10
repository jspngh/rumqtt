use std::collections::VecDeque;
use std::pin::Pin;

use flume::{bounded, Receiver, Sender};
use rumqtt_bytes::{ConnAck, ConnectReasonCode, Packet, Property, Protocol};
use tokio::time::{self, error::Elapsed, Duration, Instant};
use tokio_stream::{wrappers::IntervalStream, Stream, StreamExt};

use crate::{framed::Network, MqttOptions, MqttState, Outgoing, StateError, TransportError};

/// Critical errors during eventloop polling
#[derive(Debug, thiserror::Error)]
pub enum ConnectionError {
    #[error(transparent)]
    Transport(#[from] TransportError),
    #[error(transparent)]
    MqttState(#[from] StateError),
    #[error("Timeout")]
    Timeout(#[from] Elapsed),
    #[error("Connection refused, return code: `{0:?}`")]
    ConnectionRefused(ConnectReasonCode),
    #[error("Expected ConnAck packet, received: {0:?}")]
    NotConnAck(Packet),
    #[error("Requests done")]
    RequestsDone,
}

/// Events which can be yielded by the event loop
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    Incoming(Packet),
    Outgoing(Outgoing),
}

/// Eventloop with all the state of a connection
pub struct EventLoop<P: Protocol<Item = Packet> = rumqtt_bytes::V5> {
    /// Options of the current mqtt connection
    mqtt_options: MqttOptions,
    /// Current state of the connection
    state: MqttState,
    /// Connection to the broker
    connection: Option<Connection<P>>,
    /// Packets for transmission
    transmit: TransmitQueue,
}

struct Connection<P: Protocol<Item = Packet>> {
    /// Underlying network connection
    network: Network<P>,
    /// Stream of instants when to send a ping
    keep_alive: Pin<Box<dyn Stream<Item = Instant> + Send>>,
}

struct TransmitQueue {
    /// Requests coming from the client
    requests_rx: Receiver<Packet>,
    /// Pending packets from last session
    pending: VecDeque<Packet>,
}

impl<P: Protocol<Item = Packet>> EventLoop<P> {
    /// Create a new `EventLoop` instance and a [`Sender`] for outgoing requests.
    pub(crate) fn new(mqtt_options: MqttOptions, cap: usize) -> (Self, Sender<Packet>) {
        let (requests_tx, requests_rx) = bounded(cap);
        let max_inflight = mqtt_options.receive_max_out;
        let manual_acks = mqtt_options.manual_acks;

        let eventloop = Self {
            mqtt_options,
            state: MqttState::new(max_inflight, manual_acks),
            connection: None,
            transmit: TransmitQueue {
                requests_rx,
                pending: VecDeque::new(),
            },
        };
        (eventloop, requests_tx)
    }

    /// Last session might contain packets which aren't acked. MQTT says these packets should be
    /// republished in the next session. Move pending messages from state to eventloop, drops the
    /// underlying network connection and clears the keepalive timeout if any.
    ///
    /// > NOTE: Use only when EventLoop is blocked on network and unable to immediately handle disconnect.
    /// > Also, while this helps prevent data loss, the pending list length should be managed properly.
    /// > For this reason we recommend setting [`AsycClient`](crate::AsyncClient)'s channel capacity to `0`.
    pub fn clean(&mut self) {
        self.connection = None;
        self.transmit.clean(&mut self.state);
    }

    /// Yield next notification or outgoing request
    ///
    /// This function will progress the eventloop:
    /// - Connect/reconnect to the broker
    /// - Periodically ping the broker
    /// - Handle incoming packets
    /// - Send outgoing requests
    ///
    /// > NOTE: don't block this while iterating
    pub async fn poll(&mut self) -> Result<Event, ConnectionError> {
        if self.connection.is_none() {
            let timeout = Duration::from_secs(self.mqtt_options.connection_timeout());
            let (connection, connack) =
                time::timeout(timeout, Connection::<P>::create(&mut self.mqtt_options)).await??;
            self.connection = Some(connection);

            // Last session might contain packets which aren't acked.
            // If it's a new session, clear the pending packets.
            if !connack.session_present {
                self.transmit.pending.clear();
            }
            self.state
                .handle_incoming_packet(Packet::ConnAck(connack))?;
        }

        match self.select().await {
            Ok(v) => Ok(v),
            Err(e) => {
                // MQTT requires that packets pending acknowledgement should be republished on session resume.
                // Move pending messages from state to eventloop.
                self.clean();
                Err(e)
            }
        }
    }

    /// Perform work by `select!`ing on requests, incoming network traffic and keepalive pings
    ///
    /// ## Current flow control limitations
    /// User requests are processed if the number of inflight packets is less than
    /// the maximum number of outgoing inflight packets.
    /// However, for this to work correctly, the broker should acknowledge packets in sequence,
    /// which a lot of brokers do not do.
    ///
    /// E.g If max inflight = 5, user requests will be blocked when the inflight queue looks like
    ///   `[1, 2, 3, 4, 5]`
    ///
    /// If the broker ack's pkid 2 instead of pkid 1, a spot opens up and allows the next request
    ///   `[1, x, 3, 4, 5]`
    ///
    /// However, for the next request we will use pkid 1 again, which is still inflight.
    /// This results in a collision and the request will not be sent.
    /// The eventloop stops processing user requests until the ack for pkid 1 is received.
    async fn select(&mut self) -> Result<Event, ConnectionError> {
        // Read buffered events before creating new events
        if let Some(event) = self.state.get_event() {
            return Ok(event);
        }

        let inflight_full = self.state.inflight() >= self.mqtt_options.receive_max_out;
        let collision = self.state.has_collision();
        let allow_out = self.transmit.has_pending() || (!inflight_full && !collision);

        // We know the connection is set, since we check for `None` in the poll method
        let conn = self.connection.as_mut().expect("Connection should be set");

        tokio::select! {
            // Handles pending and new requests.
            // Only read the next user request when inflight messages are < configured inflight
            // and there are no collisions while handling previous outgoing requests.
            req = self.transmit.next(self.mqtt_options.pending_throttle), if allow_out => {
                let request = req?;
                if let Some(outgoing) = self.state.handle_outgoing_packet(request)? {
                    conn.network.write(outgoing).await?;
                }
            },
            // Read in bulk from the network, reply in bulk and yield the first item.
            // FIXME: this is probably not cancellation safe?
            res = conn.network.readb(&mut self.state) => {
                if let Err(e) = res {
                    log::error!("Error reading from network: {:?}", e);
                    return Err(ConnectionError::MqttState(e));
                }
            },
            // We generate pings irrespective of network activity. This keeps the logic simple.
            // We can change this behavior in future if necessary (to prevent extra pings)
            Some(_) = conn.keep_alive.next() => {
                let ping = Packet::PingReq(rumqtt_bytes::PingReq);
                if let Some(outgoing) = self.state.handle_outgoing_packet(ping)? {
                    conn.network.write(outgoing).await?;
                }
            }
        }

        let network_timeout = Duration::from_secs(self.mqtt_options.connection_timeout());
        time::timeout(network_timeout, conn.network.flush()).await??;

        // At least one event should be available after the work performed in the `select!`
        Ok(self.state.get_event().expect("Event should be available"))
    }
}

impl<P: Protocol<Item = Packet>> Connection<P> {
    /// Set up a network connection to the broker, and send an MQTT *connect* packet.
    async fn create(options: &mut MqttOptions) -> Result<(Self, ConnAck), ConnectionError> {
        // Create the transport layer
        let mut network = crate::transport::connect(options).await?;

        // Create the connect packet
        let mut connect = rumqtt_bytes::Connect::new(
            options.keep_alive().as_secs() as u16,
            options.clean_session(),
            options.client_id(),
        );
        connect.last_will = options.last_will().map(|x| Box::new(x.clone()));
        connect.login = options.credentials().map(|x| Box::new(x.clone()));
        connect.properties = options.connect_options.properties.clone();

        // Send the connect to the broker
        network.write(Packet::Connect(connect)).await?;
        network.flush().await?;

        // Wait for a connack response
        let connack = match network.read().await? {
            Packet::ConnAck(connack) if connack.code == ConnectReasonCode::Success => {
                for property in &connack.properties {
                    match property {
                        Property::ServerKeepAlive(keep_alive) => {
                            log::debug!("Server sets keep alive time of {keep_alive}");
                            options.keep_alive = Duration::from_secs(*keep_alive as u64);
                        }
                        Property::MaximumPacketSize(max) => {
                            log::debug!("Server sets maximum packet size of {max}");
                            network.set_max_outgoing_size(*max);
                        }
                        Property::SessionExpiryInterval(_interval) => {
                            // TODO: keep track of the interval set by the server?
                        }
                        _ => {}
                    }
                }
                connack
            }
            Packet::ConnAck(connack) => {
                return Err(ConnectionError::ConnectionRefused(connack.code))
            }
            packet => return Err(ConnectionError::NotConnAck(packet)),
        };

        let keep_alive: Pin<Box<dyn Stream<Item = Instant> + Send>> = match options.keep_alive {
            Duration::ZERO => Box::pin(tokio_stream::pending()),
            keep_alive => {
                let mut interval = time::interval_at(Instant::now() + keep_alive, keep_alive);
                interval.set_missed_tick_behavior(time::MissedTickBehavior::Delay);
                Box::pin(IntervalStream::new(interval))
            }
        };

        let connection = Self {
            network,
            keep_alive,
        };
        Ok((connection, connack))
    }
}

impl TransmitQueue {
    /// Get the next packet for transmission.
    ///
    /// This prioritises pending packets from a previous session.
    /// The `pending_throttle` parameter puts a limit on how fast pending packets are sent.
    /// If there are no pending packets, wait for the next user request.
    async fn next(&mut self, pending_throttle: Duration) -> Result<Packet, ConnectionError> {
        if !self.pending.is_empty() {
            time::sleep(pending_throttle).await;
            // We must call .pop_front() AFTER sleep() otherwise we would have
            // advanced the iterator but the future might be canceled before return
            Ok(self.pending.pop_front().unwrap())
        } else {
            match self.requests_rx.recv_async().await {
                Ok(r) => Ok(r),
                Err(_) => Err(ConnectionError::RequestsDone),
            }
        }
    }

    // TODO: better name
    fn clean(&mut self, state: &mut MqttState) {
        self.pending.extend(state.clean());

        // drain requests from channel which weren't yet received
        let requests_in_channel = self.requests_rx.drain().filter(|req| match req {
            // Wait for publish retransmission, else the broker could be confused by an unexpected ack
            Packet::PubAck(_) => false,
            _ => true,
        });
        self.pending.extend(requests_in_channel);
    }

    /// Are there any pending packets to be sent?
    fn has_pending(&self) -> bool {
        !self.pending.is_empty()
    }
}
