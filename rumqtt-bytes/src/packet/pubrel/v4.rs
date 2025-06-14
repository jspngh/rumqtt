use bytes::{BufMut, Bytes, BytesMut};

use super::PubRel;
use crate::{parse::*, Error, FixedHeader};

pub fn read(_fixed_header: FixedHeader, mut bytes: Bytes) -> Result<PubRel, Error> {
    let pkid = read_u16(&mut bytes)?;
    Ok(PubRel::new(pkid))
}

pub fn write(packet: &PubRel, buffer: &mut BytesMut) -> Result<usize, Error> {
    // packet type and flags
    buffer.put_u8(0x62);
    // remaining length
    let len = len(packet)?;
    len.write(buffer);
    // packet identifier
    buffer.put_u16(packet.pkid);

    Ok(1 + len.length() + len.value())
}

pub fn len(_packet: &PubRel) -> Result<VarInt, Error> {
    VarInt::new(2) // pkid
}
