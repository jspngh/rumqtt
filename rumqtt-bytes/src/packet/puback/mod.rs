use crate::{reason, Error, Properties};

pub(crate) mod v4;
pub(crate) mod v5;

/// Publish acknowledgement
///
/// Response to a PUBLISH packet with QoS 1.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PubAck {
    pub pkid: u16,
    pub reason: PubAckReasonCode,
    pub properties: Properties,
}

impl PubAck {
    pub fn new(pkid: u16) -> Self {
        Self {
            pkid,
            reason: PubAckReasonCode::Success,
            properties: Properties::new(),
        }
    }
}

/// Return code in puback
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PubAckReasonCode {
    Success = reason::SUCCESS,
    NoMatchingSubscribers = reason::NO_MATCHING_SUBSCRIBERS,
    UnspecifiedError = reason::UNSPECIFIED_ERROR,
    ImplementationSpecificError = reason::IMPLEMENTATION_SPECIFIC_ERROR,
    NotAuthorized = reason::NOT_AUTHORIZED,
    TopicNameInvalid = reason::TOPIC_NAME_INVALID,
    PacketIdentifierInUse = reason::PACKET_IDENTIFIER_IN_USE,
    QuotaExceeded = reason::QUOTA_EXCEEDED,
    PayloadFormatInvalid = reason::PAYLOAD_FORMAT_INVALID,
}

impl TryFrom<u8> for PubAckReasonCode {
    type Error = Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        let code = match value {
            reason::SUCCESS => Self::Success,
            reason::NO_MATCHING_SUBSCRIBERS => Self::NoMatchingSubscribers,
            reason::UNSPECIFIED_ERROR => Self::UnspecifiedError,
            reason::IMPLEMENTATION_SPECIFIC_ERROR => Self::ImplementationSpecificError,
            reason::NOT_AUTHORIZED => Self::NotAuthorized,
            reason::TOPIC_NAME_INVALID => Self::TopicNameInvalid,
            reason::PACKET_IDENTIFIER_IN_USE => Self::PacketIdentifierInUse,
            reason::QUOTA_EXCEEDED => Self::QuotaExceeded,
            reason::PAYLOAD_FORMAT_INVALID => Self::PayloadFormatInvalid,
            num => return Err(Error::InvalidConnectReturnCode(num)),
        };

        Ok(code)
    }
}
