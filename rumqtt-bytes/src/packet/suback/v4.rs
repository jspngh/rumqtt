use bytes::{Buf, BufMut, Bytes, BytesMut};

use super::SubAck;
use crate::{parse::*, Error, FixedHeader};

pub fn read(_fixed_header: FixedHeader, mut bytes: Bytes) -> Result<SubAck, Error> {
    let pkid = read_u16(&mut bytes)?;

    if !bytes.has_remaining() {
        return Err(Error::MalformedPacket);
    }

    let mut return_codes = Vec::new();
    while bytes.has_remaining() {
        let return_code = read_u8(&mut bytes)?;
        return_codes.push(return_code.try_into()?);
    }

    Ok(SubAck::new(pkid, return_codes))
}

pub fn write(packet: &SubAck, buffer: &mut BytesMut) -> Result<usize, Error> {
    // packet type and flags
    buffer.put_u8(0x90);
    // remaining length
    let len = len(packet)?;
    len.write(buffer);
    // packet identifier
    buffer.put_u16(packet.pkid);

    // return codes
    let p = packet.reason_codes.iter().map(|&c| u8::from(c));
    buffer.extend(p);

    Ok(1 + len.length() + len.value())
}

pub fn len(packet: &SubAck) -> Result<VarInt, Error> {
    let len = 2 + packet.reason_codes.len();
    VarInt::new(len)
}

#[cfg(test)]
mod test {
    use bytes::BytesMut;
    use pretty_assertions::assert_eq;

    use super::*;
    use crate::packet::V4;
    use crate::{Packet, Protocol, QoS, SubscribeReasonCode};

    #[test]
    fn suback_parsing_works() {
        let stream = vec![
            0x90, 4, // packet type, flags and remaining len
            0x00, 0x0F, // variable header. pkid = 15
            0x01, 0x80, // payload. return codes [success qos1, failure]
            0xDE, 0xAD, 0xBE, 0xEF, // extra packets in the stream
        ];

        let mut stream = BytesMut::from(&stream[..]);
        let packet = V4::read(&mut stream, 128).unwrap();

        assert_eq!(
            packet,
            Packet::SubAck(SubAck {
                pkid: 15,
                properties: None,
                reason_codes: vec![
                    SubscribeReasonCode::Success(QoS::AtLeastOnce),
                    SubscribeReasonCode::Unspecified,
                ],
            })
        );
    }
}
