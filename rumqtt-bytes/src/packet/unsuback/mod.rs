use crate::{reason, Error, Properties};

pub(crate) mod v4;
pub(crate) mod v5;

/// Unsubscribe acknowledgement
///
/// Sent by the server to the client to confirm receipt of an UNSUBSCRIBE packet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnsubAck {
    pub pkid: u16,
    pub properties: Properties,
    pub reason_codes: Vec<UnsubscribeReasonCode>,
}

impl UnsubAck {
    pub fn new(pkid: u16) -> Self {
        UnsubAck {
            pkid,
            reason_codes: Vec::new(),
            properties: Properties::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum UnsubscribeReasonCode {
    Success = reason::SUCCESS,
    NoSubscriptionExisted = reason::NO_SUBSCRIPTION_EXISTED,
    UnspecifiedError = reason::UNSPECIFIED_ERROR,
    ImplementationSpecificError = reason::IMPLEMENTATION_SPECIFIC_ERROR,
    NotAuthorized = reason::NOT_AUTHORIZED,
    TopicFilterInvalid = reason::TOPIC_FILTER_INVALID,
    PacketIdentifierInUse = reason::PACKET_IDENTIFIER_IN_USE,
}

impl TryFrom<u8> for UnsubscribeReasonCode {
    type Error = Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        let code = match value {
            reason::SUCCESS => Self::Success,
            reason::NO_SUBSCRIPTION_EXISTED => Self::NoSubscriptionExisted,
            reason::UNSPECIFIED_ERROR => Self::UnspecifiedError,
            reason::IMPLEMENTATION_SPECIFIC_ERROR => Self::ImplementationSpecificError,
            reason::NOT_AUTHORIZED => Self::NotAuthorized,
            reason::TOPIC_FILTER_INVALID => Self::TopicFilterInvalid,
            reason::PACKET_IDENTIFIER_IN_USE => Self::PacketIdentifierInUse,
            num => return Err(Error::InvalidSubscribeReasonCode(num)),
        };

        Ok(code)
    }
}
