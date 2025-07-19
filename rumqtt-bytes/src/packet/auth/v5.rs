use bytes::{BufMut, Bytes, BytesMut};

use super::Auth;
use crate::parse::*;
use crate::property::{Properties, PropertyType};
use crate::{Error, FixedHeader};

const ALLOWED_PROPERTIES: &[PropertyType] = &[
    PropertyType::AuthenticationMethod,
    PropertyType::AuthenticationData,
    PropertyType::ReasonString,
    PropertyType::UserProperty,
];

pub fn read(_fixed_header: FixedHeader, mut bytes: Bytes) -> Result<Auth, Error> {
    let code = read_u8(&mut bytes)?;
    let properties = Properties::read(&mut bytes, ALLOWED_PROPERTIES)?;
    let auth = Auth {
        code: code.try_into()?,
        properties,
    };

    Ok(auth)
}

pub fn write(packet: &Auth, buffer: &mut BytesMut) -> Result<usize, Error> {
    // packet type and flags
    buffer.put_u8(0xF0);
    // remaining length
    let len = len(packet)?;
    len.write(buffer);
    // reason code
    buffer.put_u8(packet.code as u8);
    // properties
    packet.properties.write(buffer)?;

    Ok(1 + len.length() + len.value())
}

pub fn len(packet: &Auth) -> Result<VarInt, Error> {
    let mut len = 1; // reason code

    let properties_len = packet.properties.len()?;
    len += properties_len.length() + properties_len.value();

    VarInt::new(len)
}
