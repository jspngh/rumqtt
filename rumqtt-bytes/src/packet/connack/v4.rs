use bytes::{BufMut, Bytes, BytesMut};

use super::{ConnAck, ConnectReturnCode};
use crate::parse::*;
use crate::{Error, FixedHeader};

pub fn read(_fixed_header: FixedHeader, mut bytes: Bytes) -> Result<ConnAck, Error> {
    let flags = read_u8(&mut bytes)?;
    let return_code = read_u8(&mut bytes)?;

    let session_present = (flags & 0x01) == 1;
    let code = ConnectReturnCode::try_from(return_code)?;
    Ok(ConnAck {
        session_present,
        code: code.into(),
        properties: None,
    })
}

pub fn write(packet: &ConnAck, buffer: &mut BytesMut) -> Result<usize, Error> {
    // packet type and flags
    buffer.put_u8(0x20);
    // remaining length
    let len = len(packet)?;
    len.write(buffer);
    // connect acknowledge flags
    buffer.put_u8(packet.session_present as u8);
    // return code
    buffer.put_u8(ConnectReturnCode::try_from(packet.code)? as u8);

    Ok(1 + len.length() + len.value())
}

pub fn len(_packet: &ConnAck) -> Result<VarInt, Error> {
    // sesssion present + code
    VarInt::new(1 + 1)
}
