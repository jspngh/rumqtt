use crate::{reason, Error, Properties, QoS};

pub(crate) mod v4;
pub(crate) mod v5;

/// Subscribe acknowledgement
///
/// Sent by the server to the client to confirm receipt and processing of a SUBSCRIBE packet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubAck {
    pub pkid: u16,
    pub properties: Properties,
    pub reason_codes: Vec<SubscribeReasonCode>,
}

impl SubAck {
    pub fn new(pkid: u16, reason_codes: Vec<SubscribeReasonCode>) -> Self {
        SubAck {
            pkid,
            properties: Properties::new(),
            reason_codes,
        }
    }
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
