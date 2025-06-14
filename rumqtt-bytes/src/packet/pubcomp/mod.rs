use bytes::{Buf, BufMut, Bytes, BytesMut};

use crate::parse::*;
use crate::{property::PropertyType, reason, Error};

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
    pub properties: Option<PubCompProperties>,
}

/// Return code in PubComp
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PubCompReasonCode {
    Success = reason::SUCCESS,
    PacketIdentifierNotFound = reason::PACKET_IDENTIFIER_NOT_FOUND,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PubCompProperties {
    pub reason_string: Option<String>,
    pub user_properties: Vec<(String, String)>,
}

impl PubComp {
    pub fn new(pkid: u16) -> PubComp {
        PubComp {
            pkid,
            reason: PubCompReasonCode::Success,
            properties: None,
        }
    }
}

impl PubCompProperties {
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

    pub fn read(bytes: &mut Bytes) -> Result<Option<PubCompProperties>, Error> {
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

        Ok(Some(PubCompProperties {
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
