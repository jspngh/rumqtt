use bytes::{Buf, BufMut, Bytes, BytesMut};

use crate::parse::*;
use crate::{property::PropertyType, reason, Error, QoS};

pub(crate) mod v4;
pub(crate) mod v5;

/// Subscribe acknowledgement
///
/// Sent by the server to the client to confirm receipt and processing of a SUBSCRIBE packet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubAck {
    pub pkid: u16,
    pub properties: Option<SubAckProperties>,
    pub reason_codes: Vec<SubscribeReasonCode>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubscribeReasonCode {
    Success(QoS),
    Unspecified,
    ImplementationSpecific,
    NotAuthorized,
    TopicFilterInvalid,
    PkidInUse,
    QuotaExceeded,
    SharedSubscriptionsNotSupported,
    SubscriptionIdNotSupported,
    WildcardSubscriptionsNotSupported,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubAckProperties {
    pub reason_string: Option<String>,
    pub user_properties: Vec<(String, String)>,
}

impl SubAck {
    pub fn new(pkid: u16, reason_codes: Vec<SubscribeReasonCode>) -> Self {
        SubAck {
            pkid,
            properties: None,
            reason_codes,
        }
    }
}

impl SubAckProperties {
    fn len(&self) -> Result<VarInt, Error> {
        let mut len = 0;

        if let Some(reason) = &self.reason_string {
            len += 1 + 2 + reason.len();
        }

        for (key, value) in self.user_properties.iter() {
            len += 1 + 2 + key.len() + 2 + value.len();
        }

        VarInt::new(len)
    }

    pub fn read(bytes: &mut Bytes) -> Result<Option<SubAckProperties>, Error> {
        let mut reason_string = None;
        let mut user_properties = Vec::new();

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
                PropertyType::ReasonString => {
                    let reason = read_mqtt_string(bytes)?;
                    cursor += 2 + reason.len();
                    reason_string = Some(reason);
                }
                PropertyType::UserProperty => {
                    let key = read_mqtt_string(bytes)?;
                    let value = read_mqtt_string(bytes)?;
                    cursor += 2 + key.len() + 2 + value.len();
                    user_properties.push((key, value));
                }
                _ => return Err(Error::InvalidPropertyType(prop)),
            }
        }

        Ok(Some(SubAckProperties {
            reason_string,
            user_properties,
        }))
    }

    pub fn write(&self, buffer: &mut BytesMut) -> Result<(), Error> {
        let len = self.len()?;
        len.write(buffer);

        if let Some(reason) = &self.reason_string {
            buffer.put_u8(PropertyType::ReasonString as u8);
            write_mqtt_string(buffer, reason);
        }

        for (key, value) in self.user_properties.iter() {
            buffer.put_u8(PropertyType::UserProperty as u8);
            write_mqtt_string(buffer, key);
            write_mqtt_string(buffer, value);
        }

        Ok(())
    }
}

impl TryFrom<u8> for SubscribeReasonCode {
    type Error = Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        let v = match value {
            reason::GRANTED_QOS_0 => Self::Success(QoS::AtMostOnce),
            reason::GRANTED_QOS_1 => Self::Success(QoS::AtLeastOnce),
            reason::GRANTED_QOS_2 => Self::Success(QoS::ExactlyOnce),
            reason::UNSPECIFIED_ERROR => Self::Unspecified,
            reason::IMPLEMENTATION_SPECIFIC_ERROR => Self::ImplementationSpecific,
            reason::NOT_AUTHORIZED => Self::NotAuthorized,
            reason::TOPIC_FILTER_INVALID => Self::TopicFilterInvalid,
            reason::PACKET_IDENTIFIER_IN_USE => Self::PkidInUse,
            reason::QUOTA_EXCEEDED => Self::QuotaExceeded,
            reason::SHARED_SUBSCRIPTIONS_NOT_SUPPORTED => Self::SharedSubscriptionsNotSupported,
            reason::SUBSCRIPTION_IDENTIFIERS_NOT_SUPPORTED => Self::SubscriptionIdNotSupported,
            reason::WILDCARD_SUBSCRIPTIONS_NOT_SUPPORTED => Self::WildcardSubscriptionsNotSupported,
            v => return Err(Error::InvalidSubscribeReasonCode(v)),
        };

        Ok(v)
    }
}

impl From<SubscribeReasonCode> for u8 {
    fn from(value: SubscribeReasonCode) -> u8 {
        match value {
            SubscribeReasonCode::Success(qos) => qos as u8,
            SubscribeReasonCode::Unspecified => reason::UNSPECIFIED_ERROR,
            SubscribeReasonCode::ImplementationSpecific => reason::IMPLEMENTATION_SPECIFIC_ERROR,
            SubscribeReasonCode::NotAuthorized => reason::NOT_AUTHORIZED,
            SubscribeReasonCode::TopicFilterInvalid => reason::TOPIC_FILTER_INVALID,
            SubscribeReasonCode::PkidInUse => reason::PACKET_IDENTIFIER_IN_USE,
            SubscribeReasonCode::QuotaExceeded => reason::QUOTA_EXCEEDED,
            SubscribeReasonCode::SharedSubscriptionsNotSupported => {
                reason::SHARED_SUBSCRIPTIONS_NOT_SUPPORTED
            }
            SubscribeReasonCode::SubscriptionIdNotSupported => {
                reason::SUBSCRIPTION_IDENTIFIERS_NOT_SUPPORTED
            }
            SubscribeReasonCode::WildcardSubscriptionsNotSupported => {
                reason::WILDCARD_SUBSCRIPTIONS_NOT_SUPPORTED
            }
        }
    }
}
