use bytes::{BufMut, Bytes, BytesMut};

use super::UnsubAck;
use crate::{parse::*, Error, FixedHeader};

pub fn read(fixed_header: FixedHeader, mut bytes: Bytes) -> Result<UnsubAck, Error> {
    if fixed_header.remaining_len != 2 {
        return Err(Error::PayloadSizeIncorrect);
    }

    let pkid = read_u16(&mut bytes)?;
    Ok(UnsubAck::new(pkid))
}

pub fn write(packet: &UnsubAck, buffer: &mut BytesMut) -> Result<usize, Error> {
    buffer.put_slice(&[0xB0, 0x02]);
    buffer.put_u16(packet.pkid);
    Ok(4)
}

pub fn len(_packet: &UnsubAck) -> Result<VarInt, Error> {
    VarInt::new(2) // pkid
}
