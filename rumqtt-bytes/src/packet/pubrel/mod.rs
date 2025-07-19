use crate::{reason, Error, Properties};

pub(crate) mod v4;
pub(crate) mod v5;

/// Publish release
///
/// Response to a PUBREC packet.
/// It is the third packet of the QoS 2 protocol exchange.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PubRel {
    pub pkid: u16,
    pub reason: PubRelReasonCode,
    pub properties: Properties,
}

impl PubRel {
    pub fn new(pkid: u16) -> PubRel {
        PubRel {
            pkid,
            reason: PubRelReasonCode::Success,
            properties: Properties::new(),
        }
    }
}

/// Return code in PubRel
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PubRelReasonCode {
    Success = reason::SUCCESS,
    PacketIdentifierNotFound = reason::PACKET_IDENTIFIER_NOT_FOUND,
}

impl TryFrom<u8> for PubRelReasonCode {
    type Error = Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        let code = match value {
            reason::SUCCESS => PubRelReasonCode::Success,
            reason::PACKET_IDENTIFIER_NOT_FOUND => PubRelReasonCode::PacketIdentifierNotFound,
            num => return Err(Error::InvalidConnectReturnCode(num)),
        };

        Ok(code)
    }
}
