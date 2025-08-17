use std::collections::{HashMap, VecDeque};
use std::time::Instant;

use fixedbitset::FixedBitSet;
use rumqtt_bytes::{
    ConnAck, ConnectReasonCode, Disconnect, DisconnectReasonCode, PingReq, PubAck,
    PubAckReasonCode, PubComp, PubCompReasonCode, PubRec, PubRecReasonCode, PubRel,
    PubRelReasonCode, Publish, SubAck, Subscribe, SubscribeReasonCode, UnsubAck, Unsubscribe,
    UnsubscribeReasonCode,
};
use rumqtt_bytes::{Properties, QoS};

use crate::{Event, Outgoing, Packet};

/// Errors during state handling
#[derive(Debug, thiserror::Error)]
pub enum StateError {
    /// Io Error while state is passed to network
    #[error("Io error: {0:?}")]
    Io(#[from] std::io::Error),
    #[error("Conversion error {0:?}")]
    Coversion(#[from] core::num::TryFromIntError),
    /// Invalid state for a given operation
    #[error("Invalid state for a given operation")]
    InvalidState,
    /// Received a packet (ack) which isn't asked for
    #[error("Received unsolicited ack pkid: {0}")]
    Unsolicited(u16),
    /// Last pingreq isn't acked
    #[error("Last pingreq isn't acked")]
    AwaitPingResp,
    /// Received a wrong packet while waiting for another packet
    #[error("Received a wrong packet while waiting for another packet")]
    WrongPacket,
    #[error("Timeout while waiting to resolve collision")]
    CollisionTimeout,
    #[error("A Subscribe packet must contain atleast one filter")]
    EmptySubscription,
    #[error("Mqtt serialization/deserialization error: {0}")]
    Deserialization(#[from] rumqtt_bytes::Error),
    #[error("Cannot use topic alias '{alias:?}': greater than broker maximum '{max:?}'")]
    InvalidAlias { alias: u16, max: u16 },
    #[error("Server sent disconnect with reason `{reason_string:?}` and code '{reason_code:?}'")]
    ServerDisconnect {
        reason_code: DisconnectReasonCode,
        reason_string: Option<String>,
    },
    #[error("Connection failed with reason '{reason:?}'")]
    ConnFail { reason: ConnectReasonCode },
    #[error("Connection closed by peer abruptly")]
    ConnectionAborted,
}

/// State of the mqtt connection.
// Design: Methods will just modify the state of the object without doing any network operations
// Design: All inflight queues are maintained in a pre initialized vec with index as packet id.
// This is done for 2 reasons
// Bad acks or out of order acks aren't O(n) causing cpu spikes
// Any missing acks from the broker are detected during the next recycled use of packet ids
#[derive(Debug, Clone)]
pub struct MqttState {
    /// Status of last ping
    pub await_pingresp: bool,
    /// Collision ping count. Collisions stop user requests
    /// which inturn trigger pings. Multiple pings without
    /// resolving collisions will result in error
    pub collision_ping_count: usize,
    /// Last incoming packet time
    last_incoming: Instant,
    /// Last outgoing packet time
    last_outgoing: Instant,
    /// Packet id of the last outgoing packet
    pub(crate) last_pkid: u16,
    /// Number of outgoing inflight publishes
    pub(crate) inflight: u16,
    /// Outgoing QoS 1, 2 publishes which aren't acked yet
    pub(crate) outgoing_pub: Vec<Option<Publish>>,
    /// Packet ids of released QoS 2 publishes
    pub(crate) outgoing_rel: FixedBitSet,
    /// Packet ids on incoming QoS 2 publishes
    pub(crate) incoming_pub: FixedBitSet,
    /// Last collision due to broker not acking in order
    pub collision: Option<Publish>,
    /// Buffered incoming packets
    pub events: VecDeque<Event>,
    /// Indicates if acknowledgements should be send immediately
    pub manual_acks: bool,
    /// Map of alias_id->topic
    topic_aliases: HashMap<u16, String>,
    /// `topic_alias_maximum` RECEIVED via connack packet
    pub broker_topic_alias_max: u16,
    /// Maximum number of allowed inflight QoS1 & QoS2 requests
    pub(crate) max_outgoing_inflight: u16,
    /// Upper limit on the maximum number of allowed inflight QoS1 & QoS2 requests
    max_outgoing_inflight_upper_limit: u16,
}

impl MqttState {
    /// Creates new MQTT state.
    ///
    /// Same state should be used during a connection for persistent sessions,
    /// while new state should instantiated for clean sessions.
    pub fn new(max_inflight: u16, manual_acks: bool) -> Self {
        MqttState {
            await_pingresp: false,
            collision_ping_count: 0,
            last_incoming: Instant::now(),
            last_outgoing: Instant::now(),
            last_pkid: 0,
            inflight: 0,
            // index 0 is wasted as 0 is not a valid packet id
            outgoing_pub: vec![None; max_inflight as usize + 1],
            outgoing_rel: FixedBitSet::with_capacity(max_inflight as usize + 1),
            incoming_pub: FixedBitSet::with_capacity(u16::MAX as usize + 1),
            collision: None,
            // TODO: Optimize these sizes later
            events: VecDeque::with_capacity(100),
            manual_acks,
            topic_aliases: HashMap::new(),
            // Set via CONNACK
            broker_topic_alias_max: 0,
            max_outgoing_inflight: max_inflight,
            max_outgoing_inflight_upper_limit: max_inflight,
        }
    }

    /// Consolidates handling of all incoming mqtt packets. Returns a `Notification` which for the
    /// user to consume and `Packet` which for the eventloop to put on the network
    /// E.g For incoming QoS1 publish packet, this method returns (Publish, Puback). Publish packet will
    /// be forwarded to user and Pubck packet will be written to network
    pub fn handle_incoming_packet(&mut self, packet: Packet) -> Result<Option<Packet>, StateError> {
        let mut packet = packet;
        let outgoing = match &mut packet {
            Packet::PingResp(_) => self.handle_incoming_pingresp()?,
            Packet::Publish(publish) => self.handle_incoming_publish(publish)?,
            Packet::SubAck(suback) => self.handle_incoming_suback(suback)?,
            Packet::UnsubAck(unsuback) => self.handle_incoming_unsuback(unsuback)?,
            Packet::PubAck(puback) => self.handle_incoming_puback(puback)?,
            Packet::PubRec(pubrec) => self.handle_incoming_pubrec(pubrec)?,
            Packet::PubRel(pubrel) => self.handle_incoming_pubrel(pubrel)?,
            Packet::PubComp(pubcomp) => self.handle_incoming_pubcomp(pubcomp)?,
            Packet::ConnAck(connack) => self.handle_incoming_connack(connack)?,
            Packet::Disconnect(disconn) => self.handle_incoming_disconn(disconn)?,
            _ => {
                log::error!("Invalid incoming packet = {:?}", packet);
                return Err(StateError::WrongPacket);
            }
        };

        self.events.push_back(Event::Incoming(packet));
        self.last_incoming = Instant::now();
        Ok(outgoing)
    }

    /// Consolidates handling of all outgoing mqtt packet logic. Returns a packet which should
    /// be put on to the network by the eventloop
    pub fn handle_outgoing_packet(
        &mut self,
        request: Packet,
    ) -> Result<Option<Packet>, StateError> {
        let packet = match request {
            Packet::Publish(publish) => self.outgoing_publish(publish)?,
            Packet::PubRel(pubrel) => self.outgoing_pubrel(pubrel)?,
            Packet::Subscribe(subscribe) => self.outgoing_subscribe(subscribe)?,
            Packet::Unsubscribe(unsubscribe) => self.outgoing_unsubscribe(unsubscribe)?,
            Packet::PingReq(_) => self.outgoing_ping()?,
            Packet::Disconnect(disconnect) => self.outgoing_disconnect(disconnect)?,
            Packet::PubAck(puback) => self.outgoing_puback(puback)?,
            Packet::PubRec(pubrec) => self.outgoing_pubrec(pubrec)?,
            _ => unimplemented!(),
        };

        self.last_outgoing = Instant::now();
        Ok(packet)
    }

    /// Returns inflight outgoing packets and clears internal queues
    pub fn clean(&mut self) -> Vec<Packet> {
        let mut pending = Vec::with_capacity(100);
        // remove and collect pending publishes
        for publish in self.outgoing_pub.iter_mut() {
            if let Some(publish) = publish.take() {
                let request = Packet::Publish(publish);
                pending.push(request);
            }
        }

        // remove and collect pending releases
        for pkid in self.outgoing_rel.ones() {
            let request = Packet::PubRel(PubRel::new(pkid as u16));
            pending.push(request);
        }
        self.outgoing_rel.clear();

        // remove packed ids of incoming qos2 publishes
        self.incoming_pub.clear();

        self.await_pingresp = false;
        self.collision_ping_count = 0;
        self.inflight = 0;
        pending
    }

    /// Get the next event to be processed by the event loop.
    pub fn get_event(&mut self) -> Option<Event> {
        self.events.pop_front()
    }

    /// Is there a collision on the packet identifiers for publish messages?
    ///
    /// A *collision* happens if a publish packet is supposed to use a packet identifier
    /// that is still in use because it hasn't been acknowledged yet (QoS1/QoS2).
    pub fn has_collision(&self) -> bool {
        self.collision.is_some()
    }

    /// Number of outgoing inflight publish packets.
    pub fn inflight(&self) -> u16 {
        self.inflight
    }

    fn handle_protocol_error(&mut self) -> Result<Option<Packet>, StateError> {
        // send DISCONNECT packet with REASON_CODE 0x82
        self.outgoing_disconnect(Disconnect {
            reason_code: DisconnectReasonCode::ProtocolError,
            properties: Properties::new(),
        })
    }

    fn handle_incoming_suback(
        &mut self,
        suback: &mut SubAck,
    ) -> Result<Option<Packet>, StateError> {
        for reason in suback.reason_codes.iter() {
            match reason {
                SubscribeReasonCode::Success(qos) => {
                    log::debug!("SubAck Pkid = {:?}, QoS = {:?}", suback.pkid, qos);
                }
                _ => {
                    log::warn!("SubAck Pkid = {:?}, Reason = {:?}", suback.pkid, reason);
                }
            }
        }
        Ok(None)
    }

    fn handle_incoming_unsuback(
        &mut self,
        unsuback: &mut UnsubAck,
    ) -> Result<Option<Packet>, StateError> {
        for reason in unsuback.reason_codes.iter() {
            if reason != &UnsubscribeReasonCode::Success {
                log::warn!("UnsubAck Pkid = {:?}, Reason = {:?}", unsuback.pkid, reason);
            }
        }
        Ok(None)
    }

    fn handle_incoming_connack(
        &mut self,
        connack: &mut ConnAck,
    ) -> Result<Option<Packet>, StateError> {
        if connack.code != ConnectReasonCode::Success {
            return Err(StateError::ConnFail {
                reason: connack.code,
            });
        }

        for property in &connack.properties {
            match *property {
                rumqtt_bytes::Property::TopicAliasMaximum(max) => {
                    self.broker_topic_alias_max = max;
                }
                rumqtt_bytes::Property::ReceiveMaximum(max) => {
                    self.max_outgoing_inflight = max.min(self.max_outgoing_inflight_upper_limit);
                    // FIXME: Maybe resize the pubrec and pubrel queues here to save some space.
                }
                _ => {}
            }
        }
        Ok(None)
    }

    fn handle_incoming_disconn(
        &mut self,
        disconn: &mut Disconnect,
    ) -> Result<Option<Packet>, StateError> {
        let mut reason_string = None;
        for prop in &disconn.properties {
            if let rumqtt_bytes::Property::ReasonString(reason) = prop {
                reason_string = Some(reason.clone());
            }
        }
        Err(StateError::ServerDisconnect {
            reason_code: disconn.reason_code,
            reason_string,
        })
    }

    /// Results in a publish notification in all the QoS cases.
    ///
    /// Replies with a puback in case of QoS1 and replies
    /// with a pubrec in case of QoS2 while also storing the message
    fn handle_incoming_publish(
        &mut self,
        publish: &mut Publish,
    ) -> Result<Option<Packet>, StateError> {
        for property in &publish.properties {
            // handle topic alias
            if let rumqtt_bytes::Property::TopicAlias(alias) = property {
                if !publish.topic.is_empty() {
                    self.topic_aliases.insert(*alias, publish.topic.clone());
                } else if let Some(topic) = self.topic_aliases.get(alias) {
                    topic.clone_into(&mut publish.topic);
                } else {
                    // TODO: return here?
                    self.handle_protocol_error()?;
                };
            }
        }

        // handle puback
        match publish.qos {
            QoS::AtMostOnce => (),
            QoS::AtLeastOnce => {
                if !self.manual_acks {
                    let puback = PubAck::new(publish.pkid);
                    return self.outgoing_puback(puback);
                }
            }
            QoS::ExactlyOnce => {
                let pkid = publish.pkid;
                self.incoming_pub.insert(pkid as usize);

                if !self.manual_acks {
                    let pubrec = PubRec::new(pkid);
                    return self.outgoing_pubrec(pubrec);
                }
            }
        }
        Ok(None)
    }

    fn handle_incoming_puback(&mut self, puback: &PubAck) -> Result<Option<Packet>, StateError> {
        match self.outgoing_pub.get_mut(puback.pkid as usize) {
            Some(p) if p.is_some() => p.take(),
            _ => {
                log::error!("Unsolicited puback packet: {:?}", puback.pkid);
                return Err(StateError::Unsolicited(puback.pkid));
            }
        };

        self.inflight -= 1;

        if puback.reason != PubAckReasonCode::Success
            && puback.reason != PubAckReasonCode::NoMatchingSubscribers
        {
            log::warn!(
                "PubAck Pkid = {:?}, reason: {:?}",
                puback.pkid,
                puback.reason
            );
            return Ok(None);
        }

        let packet = self.check_collision(puback.pkid).map(|publish| {
            self.outgoing_pub[publish.pkid as usize] = Some(publish.clone());
            self.inflight += 1;

            let event = Event::Outgoing(Outgoing::Publish(publish.pkid));
            self.events.push_back(event);
            self.collision_ping_count = 0;

            Packet::Publish(publish)
        });

        Ok(packet)
    }

    fn handle_incoming_pubrec(&mut self, pubrec: &PubRec) -> Result<Option<Packet>, StateError> {
        match self.outgoing_pub.get_mut(pubrec.pkid as usize) {
            Some(p) if p.is_some() => p.take(),
            _ => {
                log::error!("Unsolicited pubrec packet: {:?}", pubrec.pkid);
                return Err(StateError::Unsolicited(pubrec.pkid));
            }
        };

        if pubrec.reason != PubRecReasonCode::Success
            && pubrec.reason != PubRecReasonCode::NoMatchingSubscribers
        {
            log::warn!(
                "PubRec Pkid = {:?}, reason: {:?}",
                pubrec.pkid,
                pubrec.reason
            );
            return Ok(None);
        }

        // NOTE: Inflight - 1 for qos2 in comp
        self.outgoing_rel.insert(pubrec.pkid as usize);
        let event = Event::Outgoing(Outgoing::PubRel(pubrec.pkid));
        self.events.push_back(event);

        Ok(Some(Packet::PubRel(PubRel::new(pubrec.pkid))))
    }

    fn handle_incoming_pubrel(&mut self, pubrel: &PubRel) -> Result<Option<Packet>, StateError> {
        if !self.incoming_pub.contains(pubrel.pkid as usize) {
            log::error!("Unsolicited pubrel packet: {:?}", pubrel.pkid);
            return Err(StateError::Unsolicited(pubrel.pkid));
        }
        self.incoming_pub.set(pubrel.pkid as usize, false);

        if pubrel.reason != PubRelReasonCode::Success {
            log::warn!(
                "PubRel Pkid = {:?}, reason: {:?}",
                pubrel.pkid,
                pubrel.reason
            );
            return Ok(None);
        }

        let event = Event::Outgoing(Outgoing::PubComp(pubrel.pkid));
        self.events.push_back(event);

        Ok(Some(Packet::PubComp(PubComp::new(pubrel.pkid))))
    }

    fn handle_incoming_pubcomp(&mut self, pubcomp: &PubComp) -> Result<Option<Packet>, StateError> {
        // TODO: should this be done before or after checking outgoing_rel?
        let outgoing = self.check_collision(pubcomp.pkid).map(|publish| {
            let event = Event::Outgoing(Outgoing::Publish(publish.pkid));
            self.events.push_back(event);
            self.collision_ping_count = 0;

            Packet::Publish(publish)
        });

        if !self.outgoing_rel.contains(pubcomp.pkid as usize) {
            log::error!("Unsolicited pubcomp packet: {:?}", pubcomp.pkid);
            return Err(StateError::Unsolicited(pubcomp.pkid));
        }
        self.outgoing_rel.set(pubcomp.pkid as usize, false);

        if pubcomp.reason != PubCompReasonCode::Success {
            log::warn!(
                "PubComp Pkid = {:?}, reason: {:?}",
                pubcomp.pkid,
                pubcomp.reason
            );
            return Ok(None);
        }

        self.inflight -= 1;
        Ok(outgoing)
    }

    fn handle_incoming_pingresp(&mut self) -> Result<Option<Packet>, StateError> {
        self.await_pingresp = false;
        Ok(None)
    }

    /// Adds next packet identifier to QoS 1 and 2 publish packets
    /// and returns it by wrapping it in [Packet]
    fn outgoing_publish(&mut self, mut publish: Publish) -> Result<Option<Packet>, StateError> {
        if publish.qos != QoS::AtMostOnce {
            if publish.pkid == 0 {
                publish.pkid = self.next_pkid();
            }

            let pkid = publish.pkid;
            if self
                .outgoing_pub
                .get(publish.pkid as usize)
                .ok_or(StateError::Unsolicited(publish.pkid))? // TODO: error doesn't seem right
                .is_some()
            {
                log::info!("Collision on packet id = {:?}", publish.pkid);
                self.collision = Some(publish);
                let event = Event::Outgoing(Outgoing::AwaitAck(pkid));
                self.events.push_back(event);
                return Ok(None);
            }

            // if there is an existing publish at this pkid, this implies that broker hasn't acked this
            // packet yet. This error is possible only when broker isn't acking sequentially
            self.outgoing_pub[pkid as usize] = Some(publish.clone());
            self.inflight += 1;
        };

        log::debug!(
            "Publish. Topic = {}, Pkid = {:?}, Payload Size = {:?}",
            publish.topic,
            publish.pkid,
            publish.payload.len()
        );

        for property in &publish.properties {
            if let rumqtt_bytes::Property::TopicAlias(alias) = property {
                if *alias > self.broker_topic_alias_max {
                    // We MUST NOT send a Topic Alias that is greater than the
                    // broker's Topic Alias Maximum.
                    return Err(StateError::InvalidAlias {
                        alias: *alias,
                        max: self.broker_topic_alias_max,
                    });
                }
            }
        }

        let event = Event::Outgoing(Outgoing::Publish(publish.pkid));
        self.events.push_back(event);

        Ok(Some(Packet::Publish(publish)))
    }

    fn outgoing_pubrel(&mut self, pubrel: PubRel) -> Result<Option<Packet>, StateError> {
        let pubrel = self.save_pubrel(pubrel)?;

        log::debug!("Pubrel. Pkid = {}", pubrel.pkid);

        let event = Event::Outgoing(Outgoing::PubRel(pubrel.pkid));
        self.events.push_back(event);

        Ok(Some(Packet::PubRel(PubRel::new(pubrel.pkid))))
    }

    fn outgoing_puback(&mut self, puback: PubAck) -> Result<Option<Packet>, StateError> {
        let event = Event::Outgoing(Outgoing::PubAck(puback.pkid));
        self.events.push_back(event);

        Ok(Some(Packet::PubAck(puback)))
    }

    fn outgoing_pubrec(&mut self, pubrec: PubRec) -> Result<Option<Packet>, StateError> {
        let event = Event::Outgoing(Outgoing::PubRec(pubrec.pkid));
        self.events.push_back(event);

        Ok(Some(Packet::PubRec(pubrec)))
    }

    /// check when the last control packet/pingreq packet is received and return
    /// the status which tells if keep alive time has exceeded
    /// NOTE: status will be checked for zero keepalive times also
    fn outgoing_ping(&mut self) -> Result<Option<Packet>, StateError> {
        let elapsed_in = self.last_incoming.elapsed();
        let elapsed_out = self.last_outgoing.elapsed();

        if self.collision.is_some() {
            self.collision_ping_count += 1;
            if self.collision_ping_count >= 2 {
                return Err(StateError::CollisionTimeout);
            }
        }

        // raise error if last ping didn't receive ack
        if self.await_pingresp {
            return Err(StateError::AwaitPingResp);
        }

        self.await_pingresp = true;

        log::debug!(
            "Pingreq, last incoming packet before {:?}, last outgoing request before {:?}",
            elapsed_in,
            elapsed_out,
        );

        let event = Event::Outgoing(Outgoing::PingReq);
        self.events.push_back(event);

        Ok(Some(Packet::PingReq(PingReq)))
    }

    fn outgoing_subscribe(
        &mut self,
        mut subscription: Subscribe,
    ) -> Result<Option<Packet>, StateError> {
        if subscription.filters.is_empty() {
            return Err(StateError::EmptySubscription);
        }

        let pkid = self.next_pkid();
        subscription.pkid = pkid;

        log::debug!(
            "Subscribe. Topics = {:?}, Pkid = {:?}",
            subscription.filters,
            subscription.pkid
        );

        let event = Event::Outgoing(Outgoing::Subscribe(subscription.pkid));
        self.events.push_back(event);

        Ok(Some(Packet::Subscribe(subscription)))
    }

    fn outgoing_unsubscribe(
        &mut self,
        mut unsub: Unsubscribe,
    ) -> Result<Option<Packet>, StateError> {
        unsub.pkid = self.next_pkid();

        log::debug!(
            "Unsubscribe. Topics = {:?}, Pkid = {:?}",
            unsub.filters,
            unsub.pkid
        );

        let event = Event::Outgoing(Outgoing::Unsubscribe(unsub.pkid));
        self.events.push_back(event);

        Ok(Some(Packet::Unsubscribe(unsub)))
    }

    fn outgoing_disconnect(
        &mut self,
        disconnect: Disconnect,
    ) -> Result<Option<Packet>, StateError> {
        log::debug!("Disconnect with reason {:?}", disconnect.reason_code);
        let event = Event::Outgoing(Outgoing::Disconnect);
        self.events.push_back(event);

        Ok(Some(Packet::Disconnect(disconnect)))
    }

    fn check_collision(&mut self, pkid: u16) -> Option<Publish> {
        if let Some(publish) = &self.collision {
            if publish.pkid == pkid {
                return self.collision.take();
            }
        }

        None
    }

    fn save_pubrel(&mut self, mut pubrel: PubRel) -> Result<PubRel, StateError> {
        let pubrel = match pubrel.pkid {
            // consider PacketIdentifier(0) as uninitialized packets
            0 => {
                pubrel.pkid = self.next_pkid();
                pubrel
            }
            _ => pubrel,
        };

        self.outgoing_rel.insert(pubrel.pkid as usize);
        self.inflight += 1;
        Ok(pubrel)
    }

    /// http://stackoverflow.com/questions/11115364/mqtt-messageid-practical-implementation
    /// Packet ids are incremented till maximum set inflight messages and reset to 1 after that.
    ///
    fn next_pkid(&mut self) -> u16 {
        let next_pkid = self.last_pkid + 1;

        // When next packet id is at the edge of inflight queue,
        // set await flag. This instructs eventloop to stop
        // processing requests until all the inflight publishes
        // are acked
        if next_pkid == self.max_outgoing_inflight {
            self.last_pkid = 0;
            return next_pkid;
        }

        self.last_pkid = next_pkid;
        next_pkid
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn build_outgoing_publish(qos: QoS) -> Publish {
        let topic = "hello/world".to_owned();
        let payload = vec![1, 2, 3];

        let mut publish = Publish::new(topic, QoS::AtLeastOnce, payload);
        publish.qos = qos;
        publish
    }

    fn build_incoming_publish(qos: QoS, pkid: u16) -> Publish {
        let topic = "hello/world".to_owned();
        let payload = vec![1, 2, 3];

        let mut publish = Publish::new(topic, QoS::AtLeastOnce, payload);
        publish.pkid = pkid;
        publish.qos = qos;
        publish
    }

    fn build_mqttstate() -> MqttState {
        MqttState::new(u16::MAX, false)
    }

    #[test]
    fn next_pkid_increments_as_expected() {
        let mut mqtt = build_mqttstate();

        for i in 1..=100 {
            let pkid = mqtt.next_pkid();

            // loops between 0-99. % 100 == 0 implies border
            let expected = i % 100;
            if expected == 0 {
                break;
            }

            assert_eq!(expected, pkid);
        }
    }

    #[test]
    fn outgoing_publish_should_set_pkid_and_add_publish_to_queue() {
        let mut mqtt = build_mqttstate();

        // QoS0 Publish
        let publish = build_outgoing_publish(QoS::AtMostOnce);

        // QoS 0 publish shouldn't be saved in queue
        mqtt.outgoing_publish(publish).unwrap();
        assert_eq!(mqtt.last_pkid, 0);
        assert_eq!(mqtt.inflight, 0);

        // QoS1 Publish
        let publish = build_outgoing_publish(QoS::AtLeastOnce);

        // Packet id should be set and publish should be saved in queue
        mqtt.outgoing_publish(publish.clone()).unwrap();
        assert_eq!(mqtt.last_pkid, 1);
        assert_eq!(mqtt.inflight, 1);

        // Packet id should be incremented and publish should be saved in queue
        mqtt.outgoing_publish(publish).unwrap();
        assert_eq!(mqtt.last_pkid, 2);
        assert_eq!(mqtt.inflight, 2);

        // QoS1 Publish
        let publish = build_outgoing_publish(QoS::ExactlyOnce);

        // Packet id should be set and publish should be saved in queue
        mqtt.outgoing_publish(publish.clone()).unwrap();
        assert_eq!(mqtt.last_pkid, 3);
        assert_eq!(mqtt.inflight, 3);

        // Packet id should be incremented and publish should be saved in queue
        mqtt.outgoing_publish(publish).unwrap();
        assert_eq!(mqtt.last_pkid, 4);
        assert_eq!(mqtt.inflight, 4);
    }

    #[test]
    fn outgoing_publish_with_max_inflight_is_ok() {
        let mut mqtt = MqttState::new(2, false);

        // QoS2 publish
        let publish = build_outgoing_publish(QoS::ExactlyOnce);

        mqtt.outgoing_publish(publish.clone()).unwrap();
        assert_eq!(mqtt.last_pkid, 1);
        assert_eq!(mqtt.inflight, 1);

        // Packet id should be set back down to 0, since we hit the limit
        mqtt.outgoing_publish(publish.clone()).unwrap();
        assert_eq!(mqtt.last_pkid, 0);
        assert_eq!(mqtt.inflight, 2);

        // This should cause a collition
        mqtt.outgoing_publish(publish.clone()).unwrap();
        assert_eq!(mqtt.last_pkid, 1);
        assert_eq!(mqtt.inflight, 2);
        assert!(mqtt.collision.is_some());

        mqtt.handle_incoming_puback(&PubAck::new(1)).unwrap();
        mqtt.handle_incoming_puback(&PubAck::new(2)).unwrap();
        assert_eq!(mqtt.inflight, 1);

        // Now there should be space in the outgoing queue
        mqtt.outgoing_publish(publish.clone()).unwrap();
        assert_eq!(mqtt.last_pkid, 0);
        assert_eq!(mqtt.inflight, 2);
    }

    #[test]
    fn incoming_publish_should_be_added_to_queue_correctly() {
        let mut mqtt = build_mqttstate();

        // QoS0, 1, 2 Publishes
        let mut publish1 = build_incoming_publish(QoS::AtMostOnce, 1);
        let mut publish2 = build_incoming_publish(QoS::AtLeastOnce, 2);
        let mut publish3 = build_incoming_publish(QoS::ExactlyOnce, 3);

        mqtt.handle_incoming_publish(&mut publish1).unwrap();
        mqtt.handle_incoming_publish(&mut publish2).unwrap();
        mqtt.handle_incoming_publish(&mut publish3).unwrap();

        // only qos2 publish should be add to queue
        assert!(mqtt.incoming_pub.contains(3));
    }

    #[test]
    fn incoming_publish_should_be_acked() {
        let mut mqtt = build_mqttstate();

        // QoS0, 1, 2 Publishes
        let mut publish1 = build_incoming_publish(QoS::AtMostOnce, 1);
        let mut publish2 = build_incoming_publish(QoS::AtLeastOnce, 2);
        let mut publish3 = build_incoming_publish(QoS::ExactlyOnce, 3);

        mqtt.handle_incoming_publish(&mut publish1).unwrap();
        mqtt.handle_incoming_publish(&mut publish2).unwrap();
        mqtt.handle_incoming_publish(&mut publish3).unwrap();

        if let Event::Outgoing(Outgoing::PubAck(pkid)) = mqtt.events[0] {
            assert_eq!(pkid, 2);
        } else {
            panic!("missing puback");
        }

        if let Event::Outgoing(Outgoing::PubRec(pkid)) = mqtt.events[1] {
            assert_eq!(pkid, 3);
        } else {
            panic!("missing PubRec");
        }
    }

    #[test]
    fn incoming_publish_should_not_be_acked_with_manual_acks() {
        let mut mqtt = build_mqttstate();
        mqtt.manual_acks = true;

        // QoS0, 1, 2 Publishes
        let mut publish1 = build_incoming_publish(QoS::AtMostOnce, 1);
        let mut publish2 = build_incoming_publish(QoS::AtLeastOnce, 2);
        let mut publish3 = build_incoming_publish(QoS::ExactlyOnce, 3);

        mqtt.handle_incoming_publish(&mut publish1).unwrap();
        mqtt.handle_incoming_publish(&mut publish2).unwrap();
        mqtt.handle_incoming_publish(&mut publish3).unwrap();

        assert!(mqtt.incoming_pub.contains(3));
        assert!(mqtt.events.is_empty());
    }

    #[test]
    fn incoming_qos2_publish_should_send_rec_to_network_and_publish_to_user() {
        let mut mqtt = build_mqttstate();
        let mut publish = build_incoming_publish(QoS::ExactlyOnce, 1);

        match mqtt.handle_incoming_publish(&mut publish).unwrap().unwrap() {
            Packet::PubRec(pubrec) => assert_eq!(pubrec.pkid, 1),
            packet => panic!("Invalid network request: {:?}", packet),
        }
    }

    #[test]
    fn incoming_puback_should_remove_correct_publish_from_queue() {
        let mut mqtt = build_mqttstate();

        let publish1 = build_outgoing_publish(QoS::AtLeastOnce);
        let publish2 = build_outgoing_publish(QoS::ExactlyOnce);

        mqtt.outgoing_publish(publish1).unwrap();
        mqtt.outgoing_publish(publish2).unwrap();
        assert_eq!(mqtt.inflight, 2);

        mqtt.handle_incoming_puback(&PubAck::new(1)).unwrap();
        assert_eq!(mqtt.inflight, 1);

        mqtt.handle_incoming_puback(&PubAck::new(2)).unwrap();
        assert_eq!(mqtt.inflight, 0);

        assert!(mqtt.outgoing_pub[1].is_none());
        assert!(mqtt.outgoing_pub[2].is_none());
    }

    #[test]
    fn incoming_puback_with_pkid_greater_than_max_inflight_should_be_handled_gracefully() {
        let mut mqtt = build_mqttstate();

        let got = mqtt.handle_incoming_puback(&PubAck::new(101)).unwrap_err();

        match got {
            StateError::Unsolicited(pkid) => assert_eq!(pkid, 101),
            e => panic!("Unexpected error: {}", e),
        }
    }

    #[test]
    fn incoming_pubrec_should_release_publish_from_queue_and_add_relid_to_rel_queue() {
        let mut mqtt = build_mqttstate();

        let publish1 = build_outgoing_publish(QoS::AtLeastOnce);
        let publish2 = build_outgoing_publish(QoS::ExactlyOnce);

        let _publish_out = mqtt.outgoing_publish(publish1);
        let _publish_out = mqtt.outgoing_publish(publish2);

        mqtt.handle_incoming_pubrec(&PubRec::new(2)).unwrap();
        assert_eq!(mqtt.inflight, 2);

        // check if the remaining element's pkid is 1
        let backup = mqtt.outgoing_pub[1].clone();
        assert_eq!(backup.unwrap().pkid, 1);

        // check if the qos2 element's release pkid is 2
        assert!(mqtt.outgoing_rel.contains(2));
    }

    #[test]
    fn incoming_pubrec_should_send_release_to_network_and_nothing_to_user() {
        let mut mqtt = build_mqttstate();

        let publish = build_outgoing_publish(QoS::ExactlyOnce);
        match mqtt.outgoing_publish(publish).unwrap().unwrap() {
            Packet::Publish(publish) => assert_eq!(publish.pkid, 1),
            packet => panic!("Invalid network request: {:?}", packet),
        }

        match mqtt
            .handle_incoming_pubrec(&PubRec::new(1))
            .unwrap()
            .unwrap()
        {
            Packet::PubRel(pubrel) => assert_eq!(pubrel.pkid, 1),
            packet => panic!("Invalid network request: {:?}", packet),
        }
    }

    #[test]
    fn incoming_pubrel_should_send_comp_to_network_and_nothing_to_user() {
        let mut mqtt = build_mqttstate();
        let mut publish = build_incoming_publish(QoS::ExactlyOnce, 1);

        match mqtt.handle_incoming_publish(&mut publish).unwrap().unwrap() {
            Packet::PubRec(pubrec) => assert_eq!(pubrec.pkid, 1),
            packet => panic!("Invalid network request: {:?}", packet),
        }

        match mqtt
            .handle_incoming_pubrel(&PubRel::new(1))
            .unwrap()
            .unwrap()
        {
            Packet::PubComp(pubcomp) => assert_eq!(pubcomp.pkid, 1),
            packet => panic!("Invalid network request: {:?}", packet),
        }
    }

    #[test]
    fn incoming_pubcomp_should_release_correct_pkid_from_release_queue() {
        let mut mqtt = build_mqttstate();
        let publish = build_outgoing_publish(QoS::ExactlyOnce);

        mqtt.outgoing_publish(publish).unwrap();
        mqtt.handle_incoming_pubrec(&PubRec::new(1)).unwrap();

        mqtt.handle_incoming_pubcomp(&PubComp::new(1)).unwrap();
        assert_eq!(mqtt.inflight, 0);
    }

    #[test]
    fn outgoing_ping_handle_should_throw_errors_for_no_pingresp() {
        let mut mqtt = build_mqttstate();
        mqtt.outgoing_ping().unwrap();

        // network activity other than pingresp
        let publish = build_outgoing_publish(QoS::AtLeastOnce);
        mqtt.handle_outgoing_packet(Packet::Publish(publish))
            .unwrap();
        mqtt.handle_incoming_packet(Packet::PubAck(PubAck::new(1)))
            .unwrap();

        // should throw error because we didn't get pingresp for previous ping
        match mqtt.outgoing_ping() {
            Ok(_) => panic!("Should throw pingresp await error"),
            Err(StateError::AwaitPingResp) => (),
            Err(e) => panic!("Should throw pingresp await error. Error = {:?}", e),
        }
    }

    #[test]
    fn outgoing_ping_handle_should_succeed_if_pingresp_is_received() {
        let mut mqtt = build_mqttstate();

        // should ping
        mqtt.outgoing_ping().unwrap();
        mqtt.handle_incoming_packet(Packet::PingResp(rumqtt_bytes::PingResp))
            .unwrap();

        // should ping
        mqtt.outgoing_ping().unwrap();
    }
}
