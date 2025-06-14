use bytes::{BufMut, Bytes, BytesMut};

use super::{Auth, AuthProperties};
use crate::parse::*;
use crate::{Error, FixedHeader};

pub fn read(_fixed_header: FixedHeader, mut bytes: Bytes) -> Result<Auth, Error> {
    let code = read_u8(&mut bytes)?;
    let properties = AuthProperties::read(&mut bytes)?;
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
    if let Some(p) = &packet.properties {
        p.write(buffer)?;
    } else {
        buffer.put_u8(0);
    }

    Ok(1 + len.length() + len.value())
}

pub fn len(packet: &Auth) -> Result<VarInt, Error> {
    let mut len = 1; // reason code

    if let Some(p) = &packet.properties {
        let properties_len = p.len()?;
        len += properties_len.length() + properties_len.value();
    } else {
        len += 1; // 0 property length
    }

    VarInt::new(len)
}
