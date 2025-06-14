use bytes::{BufMut, Bytes, BytesMut};

use super::Disconnect;
use crate::{parse::*, Error, FixedHeader};

pub fn read(fixed_header: FixedHeader, _bytes: Bytes) -> Result<Disconnect, Error> {
    if fixed_header.flags() != 0x00 {
        return Err(Error::MalformedPacket);
    };

    Ok(Disconnect::new())
}

pub fn write(packet: &Disconnect, buffer: &mut BytesMut) -> Result<usize, Error> {
    // packet type and flags
    buffer.put_u8(0xE0);
    // remaining length
    let len = len(packet)?;
    len.write(buffer);

    Ok(1 + len.length() + len.value())
}

pub fn len(_packet: &Disconnect) -> Result<VarInt, Error> {
    VarInt::new(0) // no payload
}
