use bytes::{Buf, BufMut, Bytes, BytesMut};

use crate::parse::*;
use crate::{property::PropertyType, reason, Error};

pub(crate) mod v5;

/// Authentication exchange
///
/// Sent from client to server or server to client as part of an extended authentication exchange.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Auth {
    pub code: AuthReasonCode,
    pub properties: Option<AuthProperties>,
}

/// Auth packet reason code
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum AuthReasonCode {
    Success = reason::SUCCESS,
    Continue = reason::CONTINUE_AUTHENTICATION,
    ReAuthentivate = reason::RE_AUTHENTICATE,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct AuthProperties {
    pub method: Option<String>,
    pub data: Option<Bytes>,
    pub reason: Option<String>,
    pub user_properties: Vec<(String, String)>,
}

impl Auth {
    pub fn new(code: AuthReasonCode) -> Self {
        Self {
            code,
            properties: None,
        }
    }
}

impl AuthProperties {
    fn len(&self) -> Result<VarInt, Error> {
        let mut len = 0;

        if let Some(method) = &self.method {
            let m_len = method.len();
            len += 1 + m_len;
        }

        if let Some(data) = &self.data {
            let d_len = data.len();
            len += 1 + 2 + d_len;
        }

        if let Some(reason) = &self.reason {
            let r_len = reason.len();
            len += 1 + r_len;
        }

        for (key, value) in self.user_properties.iter() {
            let p_len = key.len() + value.len();
            len += 1 + p_len;
        }

        VarInt::new(len)
    }

    pub fn read(bytes: &mut Bytes) -> Result<Option<AuthProperties>, Error> {
        let properties_len = VarInt::read(bytes.iter())?;
        bytes.advance(properties_len.length());
        if properties_len == 0 {
            return Ok(None);
        }

        let mut props = AuthProperties::default();

        let mut cursor = 0;
        // read until cursor reaches property length. properties_len = 0 will skip this loop
        while properties_len > cursor {
            let prop = read_u8(bytes)?;
            cursor += 1;

            match prop.try_into()? {
                PropertyType::AuthenticationMethod => {
                    let method = read_mqtt_string(bytes)?;
                    cursor += method.len();
                    props.method = Some(method);
                }
                PropertyType::AuthenticationData => {
                    let data = read_mqtt_bytes(bytes)?;
                    cursor += 2 + data.len();
                    props.data = Some(data);
                }
                PropertyType::ReasonString => {
                    let reason = read_mqtt_string(bytes)?;
                    cursor += reason.len();
                    props.reason = Some(reason);
                }
                PropertyType::UserProperty => {
                    let key = read_mqtt_string(bytes)?;
                    let value = read_mqtt_string(bytes)?;
                    cursor += 2 + key.len() + 2 + value.len();
                    props.user_properties.push((key, value));
                }
                _ => return Err(Error::InvalidPropertyType(prop)),
            }
        }

        Ok(Some(props))
    }

    pub fn write(&self, buffer: &mut BytesMut) -> Result<(), Error> {
        let len = self.len()?;
        len.write(buffer);

        if let Some(authentication_method) = &self.method {
            buffer.put_u8(PropertyType::AuthenticationMethod as u8);
            write_mqtt_string(buffer, authentication_method);
        }

        if let Some(authentication_data) = &self.data {
            buffer.put_u8(PropertyType::AuthenticationData as u8);
            write_mqtt_bytes(buffer, authentication_data);
        }

        if let Some(reason) = &self.reason {
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

impl TryFrom<u8> for AuthReasonCode {
    type Error = Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        let code = match value {
            reason::SUCCESS => AuthReasonCode::Success,
            reason::CONTINUE_AUTHENTICATION => AuthReasonCode::Continue,
            reason::RE_AUTHENTICATE => AuthReasonCode::ReAuthentivate,
            _ => return Err(Error::MalformedPacket),
        };
        Ok(code)
    }
}
