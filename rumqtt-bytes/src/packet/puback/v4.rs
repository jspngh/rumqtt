use bytes::{BufMut, Bytes, BytesMut};

use super::PubAck;
use crate::{parse::*, Error, FixedHeader};

pub fn read(_fixed_header: FixedHeader, mut bytes: Bytes) -> Result<PubAck, Error> {
    let pkid = read_u16(&mut bytes)?;
    Ok(PubAck::new(pkid))
}

pub fn write(packet: &PubAck, buffer: &mut BytesMut) -> Result<usize, Error> {
    // packet type and flags
    buffer.put_u8(0x40);
    // remaining length
    let len = len(packet)?;
    len.write(buffer);
    // packet identifier
    buffer.put_u16(packet.pkid);

    Ok(1 + len.length() + len.value())
}

pub fn len(_packet: &PubAck) -> Result<VarInt, Error> {
    VarInt::new(2) // pkid
}

#[cfg(test)]
mod test {
    use bytes::BytesMut;
    use pretty_assertions::assert_eq;

    use super::*;
    use crate::packet::V4;
    use crate::{Packet, Protocol, PubAckReasonCode};

    #[test]
    fn puback_encoding_works() {
        let stream = &[
            0b0100_0000,
            0x02, // packet type, flags and remaining len
            0x00,
            0x0A, // fixed header. packet identifier = 10
            0xDE,
            0xAD,
            0xBE,
            0xEF, // extra packets in the stream
        ];
        let mut stream = BytesMut::from(&stream[..]);
        let packet = V4::read(&mut stream, 128).unwrap();
        assert_eq!(
            packet,
            Packet::PubAck(PubAck {
                pkid: 10,
                reason: PubAckReasonCode::Success,
                properties: None
            })
        );
    }
}
