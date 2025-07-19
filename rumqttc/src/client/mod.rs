//! High level asynchronous and synchronous interface to interact with the eventloop.

use bytes::Bytes;
use flume::Sender;

use rumqtt_bytes::{Disconnect, Filter, PubAck, PubRec, Publish, Subscribe, Unsubscribe};
use rumqtt_bytes::{Properties, Protocol};

use crate::topic::{valid_filter, valid_topic};
use crate::{v5, EventLoop, MqttOptions, Packet, QoS};

mod synchronous;
pub use synchronous::*;

/// Client Error
#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    #[error("Failed to send mqtt requests to eventloop")]
    Request(Packet),
    #[error("Failed to send mqtt requests to eventloop")]
    TryRequest(Packet),
}

impl From<flume::SendError<Packet>> for ClientError {
    fn from(e: flume::SendError<Packet>) -> Self {
        Self::Request(e.into_inner())
    }
}

impl From<flume::TrySendError<Packet>> for ClientError {
    fn from(e: flume::TrySendError<Packet>) -> Self {
        Self::TryRequest(e.into_inner())
    }
}

/// An asynchronous client, communicates with MQTT `EventLoop`.
///
/// This is cloneable and can be used to asynchronously [`publish`](`AsyncClient::publish`),
/// [`subscribe`](`AsyncClient::subscribe`) through the `EventLoop`, which is to be polled parallelly.
///
/// **NOTE**: The `EventLoop` must be regularly polled in order to send, receive and process packets
/// from the broker, i.e. move ahead.
#[derive(Clone, Debug)]
pub struct AsyncClient<P: Protocol<Item = Packet> = rumqtt_bytes::V4> {
    request_tx: Sender<Packet>,
    protocol: std::marker::PhantomData<P>,
}

impl AsyncClient {
    /// Create a new `AsyncClient` for MQTT 3.1.1 communication
    ///
    /// `cap` specifies the capacity of the bounded async channel.
    pub fn new(options: MqttOptions, cap: usize) -> (AsyncClient, EventLoop) {
        let (eventloop, request_tx) = EventLoop::new(options, cap);

        let client = AsyncClient {
            request_tx,
            protocol: std::marker::PhantomData,
        };

        (client, eventloop)
    }
}

impl v5::AsyncClient {
    /// Create a new `AsyncClient` for MQTT 5.0 communication
    ///
    /// `cap` specifies the capacity of the bounded async channel.
    pub fn new_v5(options: MqttOptions, cap: usize) -> (v5::AsyncClient, v5::EventLoop) {
        let (eventloop, request_tx) = EventLoop::new(options, cap);

        let client = v5::AsyncClient {
            request_tx,
            protocol: std::marker::PhantomData,
        };

        (client, eventloop)
    }
}

impl<P: Protocol<Item = Packet>> AsyncClient<P> {
    /// Create a new `AsyncClient` from a channel `Sender`.
    ///
    /// This is mostly useful for creating a test instance where you can
    /// listen on the corresponding receiver.
    pub fn from_senders(request_tx: Sender<Packet>) -> Self {
        Self {
            request_tx,
            protocol: std::marker::PhantomData,
        }
    }
}

impl<P: Protocol<Item = Packet>> AsyncClient<P> {
    /// Sends a MQTT Publish to the `EventLoop`.
    async fn handle_publish(
        &self,
        topic: impl Into<String>,
        qos: QoS,
        retain: bool,
        payload: impl Into<Bytes>,
        properties: Option<Properties>,
    ) -> Result<(), ClientError> {
        let topic = topic.into();
        let valid_topic = valid_topic(&topic);

        let mut publish = Publish::new(topic, qos, payload);
        publish.retain = retain;
        if let Some(properties) = properties {
            publish.properties = properties;
        }
        let publish = Packet::Publish(publish);

        if valid_topic {
            self.request_tx.send_async(publish).await?;
            Ok(())
        } else {
            Err(ClientError::Request(publish))
        }
    }

    pub async fn publish(
        &self,
        topic: impl Into<String>,
        qos: QoS,
        retain: bool,
        payload: impl Into<Bytes>,
    ) -> Result<(), ClientError> {
        self.handle_publish(topic, qos, retain, payload, None).await
    }

    /// Attempts to send a MQTT Publish to the `EventLoop`.
    fn handle_try_publish(
        &self,
        topic: impl Into<String>,
        qos: QoS,
        retain: bool,
        payload: impl Into<Bytes>,
        properties: Option<Properties>,
    ) -> Result<(), ClientError> {
        let topic = topic.into();
        let valid_topic = valid_topic(&topic);

        let mut publish = Publish::new(topic, qos, payload);
        publish.retain = retain;
        if let Some(properties) = properties {
            publish.properties = properties;
        }
        let publish = Packet::Publish(publish);

        if valid_topic {
            self.request_tx.try_send(publish)?;
            Ok(())
        } else {
            Err(ClientError::TryRequest(publish))
        }
    }

    pub fn try_publish(
        &self,
        topic: impl Into<String>,
        qos: QoS,
        retain: bool,
        payload: impl Into<Bytes>,
    ) -> Result<(), ClientError> {
        self.handle_try_publish(topic, qos, retain, payload, None)
    }
}

impl v5::AsyncClient {
    pub async fn publish_with_properties(
        &self,
        topic: impl Into<String>,
        qos: QoS,
        retain: bool,
        payload: impl Into<Bytes>,
        properties: Properties,
    ) -> Result<(), ClientError> {
        self.handle_publish(topic, qos, retain, payload, Some(properties))
            .await
    }

    pub fn try_publish_with_properties(
        &self,
        topic: impl Into<String>,
        qos: QoS,
        retain: bool,
        payload: impl Into<Bytes>,
        properties: Properties,
    ) -> Result<(), ClientError> {
        self.handle_try_publish(topic, qos, retain, payload, Some(properties))
    }
}

impl<P: Protocol<Item = Packet>> AsyncClient<P> {
    /// Sends a MQTT PubAck to the `EventLoop`.
    ///
    /// Only needed in if `manual_acks` flag is set.
    pub async fn ack(&self, publish: &Publish) -> Result<(), ClientError> {
        let ack = get_ack_req(publish);

        if let Some(ack) = ack {
            self.request_tx.send_async(ack).await?;
        }
        Ok(())
    }

    /// Attempts to send a MQTT PubAck to the `EventLoop`.
    ///
    /// Only needed in if `manual_acks` flag is set.
    pub fn try_ack(&self, publish: &Publish) -> Result<(), ClientError> {
        let ack = get_ack_req(publish);
        if let Some(ack) = ack {
            self.request_tx.try_send(ack)?;
        }
        Ok(())
    }
}

impl<P: Protocol<Item = Packet>> AsyncClient<P> {
    /// Sends a MQTT Subscribe to the `EventLoop`
    async fn handle_subscribe(
        &self,
        topic: impl Into<String>,
        qos: QoS,
        properties: Option<Properties>,
    ) -> Result<(), ClientError> {
        let filter = Filter::new(topic.into(), qos);
        let subscribe = Subscribe::new(filter, properties);
        if !subscribe_has_valid_filters(&subscribe) {
            return Err(ClientError::Request(Packet::Subscribe(subscribe)));
        }

        self.request_tx
            .send_async(Packet::Subscribe(subscribe))
            .await?;
        Ok(())
    }

    pub async fn subscribe(&self, topic: impl Into<String>, qos: QoS) -> Result<(), ClientError> {
        self.handle_subscribe(topic, qos, None).await
    }

    /// Attempts to send a MQTT Subscribe to the `EventLoop`
    fn handle_try_subscribe(
        &self,
        topic: impl Into<String>,
        qos: QoS,
        properties: Option<Properties>,
    ) -> Result<(), ClientError> {
        let filter = Filter::new(topic.into(), qos);
        let subscribe = Subscribe::new(filter, properties);
        if !subscribe_has_valid_filters(&subscribe) {
            return Err(ClientError::TryRequest(Packet::Subscribe(subscribe)));
        }

        self.request_tx.try_send(Packet::Subscribe(subscribe))?;
        Ok(())
    }

    pub fn try_subscribe(&self, topic: impl Into<String>, qos: QoS) -> Result<(), ClientError> {
        self.handle_try_subscribe(topic, qos, None)
    }

    /// Sends a MQTT Subscribe for multiple topics to the `EventLoop`
    async fn handle_subscribe_many<T>(
        &self,
        topics: T,
        properties: Option<Properties>,
    ) -> Result<(), ClientError>
    where
        T: IntoIterator<Item = Filter>,
    {
        let subscribe = Subscribe::new_many(topics, properties);
        if !subscribe_has_valid_filters(&subscribe) {
            return Err(ClientError::Request(Packet::Subscribe(subscribe)));
        }

        self.request_tx
            .send_async(Packet::Subscribe(subscribe))
            .await?;

        Ok(())
    }

    pub async fn subscribe_many<T>(&self, topics: T) -> Result<(), ClientError>
    where
        T: IntoIterator<Item = Filter>,
    {
        self.handle_subscribe_many(topics, None).await
    }

    /// Attempts to send a MQTT Subscribe for multiple topics to the `EventLoop`
    fn handle_try_subscribe_many<T>(
        &self,
        topics: T,
        properties: Option<Properties>,
    ) -> Result<(), ClientError>
    where
        T: IntoIterator<Item = Filter>,
    {
        let subscribe = Subscribe::new_many(topics, properties);
        if !subscribe_has_valid_filters(&subscribe) {
            return Err(ClientError::TryRequest(Packet::Subscribe(subscribe)));
        }

        self.request_tx.try_send(Packet::Subscribe(subscribe))?;
        Ok(())
    }

    pub fn try_subscribe_many<T>(&self, topics: T) -> Result<(), ClientError>
    where
        T: IntoIterator<Item = Filter>,
    {
        self.handle_try_subscribe_many(topics, None)
    }
}

impl v5::AsyncClient {
    pub async fn subscribe_with_properties(
        &self,
        topic: impl Into<String>,
        qos: QoS,
        properties: Properties,
    ) -> Result<(), ClientError> {
        self.handle_subscribe(topic, qos, Some(properties)).await
    }

    pub fn try_subscribe_with_properties(
        &self,
        topic: impl Into<String>,
        qos: QoS,
        properties: Properties,
    ) -> Result<(), ClientError> {
        self.handle_try_subscribe(topic, qos, Some(properties))
    }

    pub async fn subscribe_many_with_properties<T>(
        &self,
        topics: T,
        properties: Properties,
    ) -> Result<(), ClientError>
    where
        T: IntoIterator<Item = Filter>,
    {
        self.handle_subscribe_many(topics, Some(properties)).await
    }

    pub fn try_subscribe_many_with_properties<T>(
        &self,
        topics: T,
        properties: Properties,
    ) -> Result<(), ClientError>
    where
        T: IntoIterator<Item = Filter>,
    {
        self.handle_try_subscribe_many(topics, Some(properties))
    }
}

impl<P: Protocol<Item = Packet>> AsyncClient<P> {
    /// Sends a MQTT Unsubscribe to the `EventLoop`
    async fn handle_unsubscribe(
        &self,
        topic: impl Into<String>,
        properties: Option<Properties>,
    ) -> Result<(), ClientError> {
        let mut unsubscribe = Unsubscribe::new(topic);
        if let Some(properties) = properties {
            unsubscribe.properties = properties;
        }
        self.request_tx
            .send_async(Packet::Unsubscribe(unsubscribe))
            .await?;
        Ok(())
    }

    pub async fn unsubscribe<S: Into<String>>(&self, topic: S) -> Result<(), ClientError> {
        self.handle_unsubscribe(topic, None).await
    }

    /// Attempts to send a MQTT Unsubscribe to the `EventLoop`
    fn handle_try_unsubscribe<S: Into<String>>(
        &self,
        topic: S,
        properties: Option<Properties>,
    ) -> Result<(), ClientError> {
        let mut unsubscribe = Unsubscribe::new(topic);
        if let Some(properties) = properties {
            unsubscribe.properties = properties;
        }
        self.request_tx.try_send(Packet::Unsubscribe(unsubscribe))?;
        Ok(())
    }

    pub fn try_unsubscribe<S: Into<String>>(&self, topic: S) -> Result<(), ClientError> {
        self.handle_try_unsubscribe(topic, None)
    }
}

impl v5::AsyncClient {
    pub async fn unsubscribe_with_properties<S: Into<String>>(
        &self,
        topic: S,
        properties: Properties,
    ) -> Result<(), ClientError> {
        self.handle_unsubscribe(topic, Some(properties)).await
    }

    pub fn try_unsubscribe_with_properties<S: Into<String>>(
        &self,
        topic: S,
        properties: Properties,
    ) -> Result<(), ClientError> {
        self.handle_try_unsubscribe(topic, Some(properties))
    }
}

impl<P: Protocol<Item = Packet>> AsyncClient<P> {
    /// Sends a MQTT disconnect to the `EventLoop`
    pub async fn disconnect(&self) -> Result<(), ClientError> {
        let request = Packet::Disconnect(Disconnect::new());
        self.request_tx.send_async(request).await?;
        Ok(())
    }

    /// Attempts to send a MQTT disconnect to the `EventLoop`
    pub fn try_disconnect(&self) -> Result<(), ClientError> {
        let request = Packet::Disconnect(Disconnect::new());
        self.request_tx.try_send(request)?;
        Ok(())
    }
}

fn get_ack_req(publish: &Publish) -> Option<Packet> {
    let ack = match publish.qos {
        QoS::AtMostOnce => return None,
        QoS::AtLeastOnce => Packet::PubAck(PubAck::new(publish.pkid)),
        QoS::ExactlyOnce => Packet::PubRec(PubRec::new(publish.pkid)),
    };
    Some(ack)
}

#[must_use]
fn subscribe_has_valid_filters(subscribe: &Subscribe) -> bool {
    !subscribe.filters.is_empty()
        && subscribe
            .filters
            .iter()
            .all(|filter| valid_filter(&filter.path))
}

#[cfg(test)]
mod test {
    use super::*;

    use crate::OptionBuilder;
    use rumqtt_bytes::LastWill;

    #[test]
    fn calling_iter_twice_on_connection_shouldnt_panic() {
        use std::time::Duration;

        let will = LastWill::new("hello/world", "good bye", QoS::AtMostOnce, false);
        let options = OptionBuilder::new_tcp("localhost", 1883)
            .client_id("test-1")
            .keep_alive(Duration::from_secs(5))
            .last_will(will)
            .finalize();

        let (_, mut connection) = Client::new(options, 10);
        let _ = connection.iter();
        let _ = connection.iter();
    }

    #[test]
    fn should_be_able_to_build_test_client_from_channel() {
        let (tx, rx) = flume::bounded(1);
        let client = v5::Client::from_sender(tx);
        client
            .publish("hello/world", QoS::ExactlyOnce, false, "good bye")
            .expect("Should be able to publish");
        let _ = rx.try_recv().expect("Should have message");
    }
}
