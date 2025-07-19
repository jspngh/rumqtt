use crate::{reason, Error, Properties};

pub(crate) mod v4;
pub(crate) mod v5;

/// Publish complete
///
/// Response to a PUBREL packet.
/// It is the fourth and final packet of the QoS 2 protocol exchange.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PubComp {
    pub pkid: u16,
    pub reason: PubCompReasonCode,
    pub properties: Properties,
}

impl PubComp {
    pub fn new(pkid: u16) -> PubComp {
        PubComp {
            pkid,
            reason: PubCompReasonCode::Success,
            properties: Properties::new(),
        }
    }
}

/// Return code in PubComp
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PubCompReasonCode {
    Success = reason::SUCCESS,
    PacketIdentifierNotFound = reason::PACKET_IDENTIFIER_NOT_FOUND,
}

impl TryFrom<u8> for PubCompReasonCode {
    type Error = Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        let code = match value {
            reason::SUCCESS => PubCompReasonCode::Success,
            reason::PACKET_IDENTIFIER_NOT_FOUND => PubCompReasonCode::PacketIdentifierNotFound,
            num => return Err(Error::InvalidConnectReturnCode(num)),
        };

        Ok(code)
    }
}
