use crate::{reason, Error, Properties};

pub(crate) mod v4;
pub(crate) mod v5;

/// Publish received
///
/// Response to a PUBLISH packet with QoS 2.
/// It is the second packet of the QoS 2 protocol exchange.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PubRec {
    pub pkid: u16,
    pub reason: PubRecReasonCode,
    pub properties: Properties,
}

impl PubRec {
    pub fn new(pkid: u16) -> PubRec {
        PubRec {
            pkid,
            reason: PubRecReasonCode::Success,
            properties: Properties::new(),
        }
    }
}

/// Return code in PubRec
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PubRecReasonCode {
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

impl TryFrom<u8> for PubRecReasonCode {
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
