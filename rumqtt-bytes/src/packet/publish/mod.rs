use bytes::{Buf, BufMut, Bytes, BytesMut};

use crate::parse::*;
use crate::{property::PropertyType, Error, QoS};

pub(crate) mod v4;
pub(crate) mod v5;

/// Publish message
///
/// Sent from a client to a server or from a server to a client to transport an application message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Publish {
    pub dup: bool,
    pub qos: QoS,
    pub retain: bool,
    pub topic: String,
    pub pkid: u16,
    pub properties: Option<PublishProperties>,
    pub payload: Bytes,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct PublishProperties {
    pub payload_format_indicator: Option<u8>,
    pub message_expiry_interval: Option<u32>,
    pub topic_alias: Option<u16>,
    pub response_topic: Option<String>,
    pub correlation_data: Option<Bytes>,
    pub user_properties: Vec<(String, String)>,
    pub subscription_ids: Vec<VarInt>,
    pub content_type: Option<String>,
}

impl Publish {
    pub fn new<T: Into<String>, P: Into<Bytes>>(topic: T, qos: QoS, payload: P) -> Self {
        Publish {
            dup: false,
            qos,
            retain: false,
            pkid: 0,
            topic: topic.into(),
            payload: payload.into(),
            properties: None,
        }
    }
}

impl PublishProperties {
    fn len(&self) -> Result<VarInt, Error> {
        let mut len = 0;

        if self.payload_format_indicator.is_some() {
            len += 1 + 1;
        }

        if self.message_expiry_interval.is_some() {
            len += 1 + 4;
        }

        if self.topic_alias.is_some() {
            len += 1 + 2;
        }

        if let Some(topic) = &self.response_topic {
            len += 1 + 2 + topic.len()
        }

        if let Some(data) = &self.correlation_data {
            len += 1 + 2 + data.len()
        }

        for (key, value) in self.user_properties.iter() {
            len += 1 + 2 + key.len() + 2 + value.len();
        }

        for id in self.subscription_ids.iter() {
            len += 1 + id.length();
        }

        if let Some(typ) = &self.content_type {
            len += 1 + 2 + typ.len()
        }

        VarInt::new(len)
    }

    pub fn read(bytes: &mut Bytes) -> Result<Option<PublishProperties>, Error> {
        let mut payload_format_indicator = None;
        let mut message_expiry_interval = None;
        let mut topic_alias = None;
        let mut response_topic = None;
        let mut correlation_data = None;
        let mut user_properties = Vec::new();
        let mut subscription_ids = Vec::new();
        let mut content_type = None;

        let properties_len = VarInt::read(bytes.iter())?;
        bytes.advance(properties_len.length());
        if properties_len == 0 {
            return Ok(None);
        }

        let mut cursor = 0;
        // read until cursor reaches property length. properties_len = 0 will skip this loop
        while properties_len > cursor {
            let prop = read_u8(bytes)?;
            cursor += 1;

            match prop.try_into()? {
                PropertyType::PayloadFormatIndicator => {
                    payload_format_indicator = Some(read_u8(bytes)?);
                    cursor += 1;
                }
                PropertyType::MessageExpiryInterval => {
                    message_expiry_interval = Some(read_u32(bytes)?);
                    cursor += 4;
                }
                PropertyType::TopicAlias => {
                    topic_alias = Some(read_u16(bytes)?);
                    cursor += 2;
                }
                PropertyType::ResponseTopic => {
                    let topic = read_mqtt_string(bytes)?;
                    cursor += 2 + topic.len();
                    response_topic = Some(topic);
                }
                PropertyType::CorrelationData => {
                    let data = read_mqtt_bytes(bytes)?;
                    cursor += 2 + data.len();
                    correlation_data = Some(data);
                }
                PropertyType::UserProperty => {
                    let key = read_mqtt_string(bytes)?;
                    let value = read_mqtt_string(bytes)?;
                    cursor += 2 + key.len() + 2 + value.len();
                    user_properties.push((key, value));
                }
                PropertyType::SubscriptionIdentifier => {
                    let id = VarInt::read(bytes.iter())?;
                    cursor += id.length();
                    bytes.advance(id.length());
                    subscription_ids.push(id);
                }
                PropertyType::ContentType => {
                    let typ = read_mqtt_string(bytes)?;
                    cursor += 2 + typ.len();
                    content_type = Some(typ);
                }
                _ => return Err(Error::InvalidPropertyType(prop)),
            }
        }

        Ok(Some(PublishProperties {
            payload_format_indicator,
            message_expiry_interval,
            topic_alias,
            response_topic,
            correlation_data,
            user_properties,
            subscription_ids,
            content_type,
        }))
    }

    pub fn write(&self, buffer: &mut BytesMut) -> Result<(), Error> {
        let len = self.len()?;
        len.write(buffer);

        if let Some(payload_format_indicator) = self.payload_format_indicator {
            buffer.put_u8(PropertyType::PayloadFormatIndicator as u8);
            buffer.put_u8(payload_format_indicator);
        }

        if let Some(message_expiry_interval) = self.message_expiry_interval {
            buffer.put_u8(PropertyType::MessageExpiryInterval as u8);
            buffer.put_u32(message_expiry_interval);
        }

        if let Some(topic_alias) = self.topic_alias {
            buffer.put_u8(PropertyType::TopicAlias as u8);
            buffer.put_u16(topic_alias);
        }

        if let Some(topic) = &self.response_topic {
            buffer.put_u8(PropertyType::ResponseTopic as u8);
            write_mqtt_string(buffer, topic);
        }

        if let Some(data) = &self.correlation_data {
            buffer.put_u8(PropertyType::CorrelationData as u8);
            write_mqtt_bytes(buffer, data);
        }

        for (key, value) in self.user_properties.iter() {
            buffer.put_u8(PropertyType::UserProperty as u8);
            write_mqtt_string(buffer, key);
            write_mqtt_string(buffer, value);
        }

        for id in self.subscription_ids.iter() {
            buffer.put_u8(PropertyType::SubscriptionIdentifier as u8);
            id.write(buffer);
        }

        if let Some(typ) = &self.content_type {
            buffer.put_u8(PropertyType::ContentType as u8);
            write_mqtt_string(buffer, typ);
        }

        Ok(())
    }
}
