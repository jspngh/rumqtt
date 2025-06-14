use bytes::{BufMut, Bytes, BytesMut};

use super::PubRec;
use crate::{parse::*, Error, FixedHeader};

pub fn read(_fixed_header: FixedHeader, mut bytes: Bytes) -> Result<PubRec, Error> {
    let pkid = read_u16(&mut bytes)?;
    Ok(PubRec::new(pkid))
}

pub fn write(packet: &PubRec, buffer: &mut BytesMut) -> Result<usize, Error> {
    // packet type and flags
    buffer.put_u8(0x50);
    let len = len(packet)?;
    len.write(buffer);
    // packet identifier
    buffer.put_u16(packet.pkid);

    Ok(1 + len.length() + len.value())
}

pub fn len(_packet: &PubRec) -> Result<VarInt, Error> {
    VarInt::new(2) // pkid
}
