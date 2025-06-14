use bytes::{BufMut, Bytes, BytesMut};

use super::PubComp;
use crate::{parse::*, Error, FixedHeader};

pub fn read(_fixed_header: FixedHeader, mut bytes: Bytes) -> Result<PubComp, Error> {
    let pkid = read_u16(&mut bytes)?;
    Ok(PubComp::new(pkid))
}

pub fn write(packet: &PubComp, buffer: &mut BytesMut) -> Result<usize, Error> {
    // packet type and flags
    buffer.put_u8(0x70);
    // remaining length
    let len = len(packet)?;
    len.write(buffer);
    // packet identifier
    buffer.put_u16(packet.pkid);

    Ok(1 + len.length() + len.value())
}

pub fn len(_packet: &PubComp) -> Result<VarInt, Error> {
    VarInt::new(2) // pkid
}
