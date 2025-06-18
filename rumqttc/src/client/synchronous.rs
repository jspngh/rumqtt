//! TODO

use std::time::Duration;

use bytes::Bytes;
use flume::Sender;
use futures_util::FutureExt;
use tokio::runtime::{self, Runtime};

use rumqtt_bytes::Protocol;
use rumqtt_bytes::{
    Disconnect, Filter, Publish, PublishProperties, Subscribe, SubscribeProperties, Unsubscribe,
    UnsubscribeProperties,
};

use super::{get_ack_req, subscribe_has_valid_filters};
use super::{AsyncClient, ClientError, RecvError, RecvTimeoutError, TryRecvError};
use crate::topic::valid_topic;
use crate::{ConnectionError, Event, EventLoop, MqttOptions, Packet, QoS};

/// Synchronous client for MQTT 3.1.1 (protocol version 4)
pub type ClientV4 = Client<rumqtt_bytes::V4>;
/// Connection for MQTT 3.1.1 (protocol version 4)
pub type ConnectionV4 = Connection<rumqtt_bytes::V4>;

/// Synchronous client for MQTT 5.0 (protocol version 5)
pub type ClientV5 = Client<rumqtt_bytes::V5>;
/// Connection for MQTT 5.0 (protocol version 5)
pub type ConnectionV5 = Connection<rumqtt_bytes::V5>;

/// A synchronous client, communicates with MQTT `EventLoop`.
///
/// This is cloneable and can be used to synchronously [`publish`](`AsyncClient::publish`),
/// [`subscribe`](`AsyncClient::subscribe`) through the `EventLoop`/`Connection`, which is to be polled in parallel
/// by iterating over the object returned by [`Connection.iter()`](Connection::iter) in a separate thread.
///
/// **NOTE**: The `EventLoop`/`Connection` must be regularly polled(`.next()` in case of `Connection`) in order
/// to send, receive and process packets from the broker, i.e. move ahead.
///
/// An asynchronous channel handle can also be extracted if necessary.
#[derive(Clone)]
pub struct Client<P: Protocol<Item = Packet>> {
    client: AsyncClient<P>,
}

impl ClientV4 {
    /// Create a new `Client`
    ///
    /// `cap` specifies the capacity of the bounded async channel.
    pub fn new(options: MqttOptions, cap: usize) -> (ClientV4, ConnectionV4) {
        let (client, eventloop) = AsyncClient::new(options, cap);
        let client = Client { client };

        let runtime = runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();

        let connection = Connection::new(eventloop, runtime);
        (client, connection)
    }
}

impl ClientV5 {
    /// Create a new `Client`
    ///
    /// `cap` specifies the capacity of the bounded async channel.
    pub fn new_v5(options: MqttOptions, cap: usize) -> (ClientV5, ConnectionV5) {
        let (client, eventloop) = AsyncClient::new_v5(options, cap);
        let client = Client { client };

        let runtime = runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();

        let connection = Connection::new(eventloop, runtime);
        (client, connection)
    }
}

impl<P: Protocol<Item = Packet>> Client<P> {
    /// Create a new `Client` from a channel `Sender`.
    ///
    /// This is mostly useful for creating a test instance where you can
    /// listen on the corresponding receiver.
    pub fn from_sender(request_tx: Sender<Packet>) -> Self {
        Client {
            client: AsyncClient::from_senders(request_tx),
        }
    }

    /// Sends a MQTT Publish to the `EventLoop`
    fn handle_publish(
        &self,
        topic: impl Into<String>,
        qos: QoS,
        retain: bool,
        payload: impl Into<Bytes>,
        properties: Option<PublishProperties>,
    ) -> Result<(), ClientError> {
        let topic = topic.into();
        let valid_topic = valid_topic(&topic);

        let mut publish = Publish::new(topic, qos, payload);
        publish.retain = retain;
        publish.properties = properties;
        let publish = Packet::Publish(publish);

        if valid_topic {
            self.client.request_tx.send(publish)?;
            Ok(())
        } else {
            Err(ClientError::Request(publish))
        }
    }

    pub fn publish(
        &self,
        topic: impl Into<String>,
        qos: QoS,
        retain: bool,
        payload: impl Into<Bytes>,
    ) -> Result<(), ClientError> {
        self.handle_publish(topic, qos, retain, payload, None)
    }

    pub fn try_publish(
        &self,
        topic: impl Into<String>,
        qos: QoS,
        retain: bool,
        payload: impl Into<Bytes>,
    ) -> Result<(), ClientError> {
        self.client.try_publish(topic, qos, retain, payload)
    }
}

impl ClientV5 {
    pub fn publish_with_properties(
        &self,
        topic: impl Into<String>,
        qos: QoS,
        retain: bool,
        payload: impl Into<Bytes>,
        properties: PublishProperties,
    ) -> Result<(), ClientError> {
        self.handle_publish(topic, qos, retain, payload, Some(properties))
    }

    pub fn try_publish_with_properties(
        &self,
        topic: impl Into<String>,
        qos: QoS,
        retain: bool,
        payload: impl Into<Bytes>,
        properties: PublishProperties,
    ) -> Result<(), ClientError> {
        self.client
            .try_publish_with_properties(topic, qos, retain, payload, properties)
    }
}

impl<P: Protocol<Item = Packet>> Client<P> {
    /// Sends a MQTT PubAck to the `EventLoop`. Only needed in if `manual_acks` flag is set.
    pub fn ack(&self, publish: &Publish) -> Result<(), ClientError> {
        let ack = get_ack_req(publish);

        if let Some(ack) = ack {
            self.client.request_tx.send(ack)?;
        }
        Ok(())
    }

    /// Sends a MQTT PubAck to the `EventLoop`. Only needed in if `manual_acks` flag is set.
    pub fn try_ack(&self, publish: &Publish) -> Result<(), ClientError> {
        self.client.try_ack(publish)?;
        Ok(())
    }
}

impl<P: Protocol<Item = Packet>> Client<P> {
    /// Sends a MQTT Subscribe to the `EventLoop`
    fn handle_subscribe(
        &self,
        topic: impl Into<String>,
        qos: QoS,
        properties: Option<SubscribeProperties>,
    ) -> Result<(), ClientError> {
        let filter = Filter::new(topic.into(), qos);
        let subscribe = Subscribe::new(filter, properties);
        if !subscribe_has_valid_filters(&subscribe) {
            return Err(ClientError::Request(Packet::Subscribe(subscribe)));
        }

        self.client.request_tx.send(Packet::Subscribe(subscribe))?;
        Ok(())
    }

    pub fn subscribe<S: Into<String>>(&self, topic: S, qos: QoS) -> Result<(), ClientError> {
        self.handle_subscribe(topic, qos, None)
    }

    pub fn try_subscribe<S: Into<String>>(&self, topic: S, qos: QoS) -> Result<(), ClientError> {
        self.client.try_subscribe(topic, qos)
    }

    /// Sends a MQTT Subscribe for multiple topics to the `EventLoop`
    fn handle_subscribe_many<T>(
        &self,
        topics: T,
        properties: Option<SubscribeProperties>,
    ) -> Result<(), ClientError>
    where
        T: IntoIterator<Item = Filter>,
    {
        let subscribe = Subscribe::new_many(topics, properties);
        if !subscribe_has_valid_filters(&subscribe) {
            return Err(ClientError::Request(Packet::Subscribe(subscribe)));
        }

        self.client.request_tx.send(Packet::Subscribe(subscribe))?;
        Ok(())
    }

    pub fn subscribe_many<T>(&self, topics: T) -> Result<(), ClientError>
    where
        T: IntoIterator<Item = Filter>,
    {
        self.handle_subscribe_many(topics, None)
    }

    pub fn try_subscribe_many<T>(&self, topics: T) -> Result<(), ClientError>
    where
        T: IntoIterator<Item = Filter>,
    {
        self.client.try_subscribe_many(topics)
    }
}

impl ClientV5 {
    pub fn subscribe_with_properties(
        &self,
        topic: impl Into<String>,
        qos: QoS,
        properties: SubscribeProperties,
    ) -> Result<(), ClientError> {
        self.handle_subscribe(topic, qos, Some(properties))
    }

    /// Sends a MQTT Subscribe to the `EventLoop`
    pub fn try_subscribe_with_properties(
        &self,
        topic: impl Into<String>,
        qos: QoS,
        properties: SubscribeProperties,
    ) -> Result<(), ClientError> {
        self.client
            .try_subscribe_with_properties(topic, qos, properties)
    }

    pub fn subscribe_many_with_properties<T>(
        &self,
        topics: T,
        properties: SubscribeProperties,
    ) -> Result<(), ClientError>
    where
        T: IntoIterator<Item = Filter>,
    {
        self.handle_subscribe_many(topics, Some(properties))
    }

    pub fn try_subscribe_many_with_properties<T>(
        &self,
        topics: T,
        properties: SubscribeProperties,
    ) -> Result<(), ClientError>
    where
        T: IntoIterator<Item = Filter>,
    {
        self.client
            .try_subscribe_many_with_properties(topics, properties)
    }
}

impl<P: Protocol<Item = Packet>> Client<P> {
    /// Sends a MQTT Unsubscribe to the `EventLoop`
    fn handle_unsubscribe(
        &self,
        topic: impl Into<String>,
        properties: Option<UnsubscribeProperties>,
    ) -> Result<(), ClientError> {
        let mut unsubscribe = Unsubscribe::new(topic);
        unsubscribe.properties = properties;
        self.client
            .request_tx
            .send(Packet::Unsubscribe(unsubscribe))?;
        Ok(())
    }

    pub fn unsubscribe(&self, topic: impl Into<String>) -> Result<(), ClientError> {
        self.handle_unsubscribe(topic, None)
    }

    pub fn try_unsubscribe(&self, topic: impl Into<String>) -> Result<(), ClientError> {
        self.client.try_unsubscribe(topic)
    }
}

impl ClientV5 {
    pub fn unsubscribe_with_properties<S: Into<String>>(
        &self,
        topic: S,
        properties: UnsubscribeProperties,
    ) -> Result<(), ClientError> {
        self.handle_unsubscribe(topic, Some(properties))
    }

    /// Sends a MQTT Unsubscribe to the `EventLoop`
    pub fn try_unsubscribe_with_properties<S: Into<String>>(
        &self,
        topic: S,
        properties: UnsubscribeProperties,
    ) -> Result<(), ClientError> {
        self.client
            .try_unsubscribe_with_properties(topic, properties)
    }
}

impl<P: Protocol<Item = Packet>> Client<P> {
    /// Sends a MQTT disconnect to the `EventLoop`
    pub fn disconnect(&self) -> Result<(), ClientError> {
        let request = Packet::Disconnect(Disconnect::new());
        self.client.request_tx.send(request)?;
        Ok(())
    }

    /// Sends a MQTT disconnect to the `EventLoop`
    pub fn try_disconnect(&self) -> Result<(), ClientError> {
        self.client.try_disconnect()?;
        Ok(())
    }
}

///  MQTT connection. Maintains all the necessary state
pub struct Connection<P: Protocol<Item = Packet>> {
    pub eventloop: EventLoop<P>,
    runtime: Runtime,
}
impl<P: Protocol<Item = Packet>> Connection<P> {
    fn new(eventloop: EventLoop<P>, runtime: Runtime) -> Self {
        Self { eventloop, runtime }
    }

    /// Returns an iterator over this connection. Iterating over this is all that's
    /// necessary to make connection progress and maintain a robust connection.
    /// Just continuing to loop will reconnect
    /// **NOTE** Don't block this while iterating
    // ideally this should be named iter_mut because it requires a mutable reference
    // Also we can implement IntoIter for this to make it easy to iterate over it
    #[must_use = "Connection should be iterated over a loop to make progress"]
    pub fn iter(&mut self) -> Iter<'_, P> {
        Iter { connection: self }
    }

    /// Attempt to fetch an incoming [`Event`] on the [`EventLoop`], returning an error
    /// if all clients/users have closed requests channel.
    ///
    /// [`EventLoop`]: super::EventLoop
    pub fn recv(&mut self) -> Result<Result<Event, ConnectionError>, RecvError> {
        let f = self.eventloop.poll();
        let event = self.runtime.block_on(f);

        resolve_event(event).ok_or(RecvError)
    }

    /// Attempt to fetch an incoming [`Event`] on the [`EventLoop`], returning an error
    /// if none immediately present or all clients/users have closed requests channel.
    ///
    /// [`EventLoop`]: super::EventLoop
    pub fn try_recv(&mut self) -> Result<Result<Event, ConnectionError>, TryRecvError> {
        let f = self.eventloop.poll();
        // Enters the runtime context so we can poll the future, as required by `now_or_never()`.
        // ref: https://docs.rs/tokio/latest/tokio/runtime/struct.Runtime.html#method.enter
        let _guard = self.runtime.enter();
        let event = f.now_or_never().ok_or(TryRecvError::Empty)?;

        resolve_event(event).ok_or(TryRecvError::Disconnected)
    }

    /// Attempt to fetch an incoming [`Event`] on the [`EventLoop`], returning an error
    /// if all clients/users have closed requests channel or the timeout has expired.
    ///
    /// [`EventLoop`]: super::EventLoop
    pub fn recv_timeout(
        &mut self,
        duration: Duration,
    ) -> Result<Result<Event, ConnectionError>, RecvTimeoutError> {
        let f = self.eventloop.poll();
        let event = self
            .runtime
            .block_on(async { tokio::time::timeout(duration, f).await })
            .map_err(|_| RecvTimeoutError::Timeout)?;

        resolve_event(event).ok_or(RecvTimeoutError::Disconnected)
    }
}

fn resolve_event(event: Result<Event, ConnectionError>) -> Option<Result<Event, ConnectionError>> {
    match event {
        Ok(v) => Some(Ok(v)),
        // closing of request channel should stop the iterator
        Err(ConnectionError::RequestsDone) => {
            log::trace!("Done with requests");
            None
        }
        Err(e) => Some(Err(e)),
    }
}

/// Iterator which polls the `EventLoop` for connection progress
pub struct Iter<'a, P: Protocol<Item = Packet>> {
    connection: &'a mut Connection<P>,
}

impl<P: Protocol<Item = Packet>> Iterator for Iter<'_, P> {
    type Item = Result<Event, ConnectionError>;

    fn next(&mut self) -> Option<Self::Item> {
        self.connection.recv().ok()
    }
}
