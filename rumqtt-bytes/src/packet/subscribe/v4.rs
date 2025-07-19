use bytes::{Buf, BufMut, Bytes, BytesMut};

use super::{Filter, RetainForwardRule, Subscribe};
use crate::{parse::*, Error, FixedHeader, Properties};

pub fn read(_fixed_header: FixedHeader, mut bytes: Bytes) -> Result<Subscribe, Error> {
    let pkid = read_u16(&mut bytes)?;

    let mut filters = Vec::new();
    while bytes.has_remaining() {
        let path = read_mqtt_string(&mut bytes)?;
        let options = read_u8(&mut bytes)?;
        let requested_qos = options & 0b0000_0011;

        filters.push(Filter {
            path,
            qos: requested_qos.try_into()?,
            nolocal: false,
            preserve_retain: false,
            retain_forward_rule: RetainForwardRule::OnEverySubscribe,
        });
    }

    match filters.len() {
        0 => Err(Error::EmptySubscription),
        _ => Ok(Subscribe {
            pkid,
            filters,
            properties: Properties::new(),
        }),
    }
}

pub fn write(packet: &Subscribe, buffer: &mut BytesMut) -> Result<usize, Error> {
    // packet type and flags
    buffer.put_u8(0x82);
    // remaining length
    let len = len(packet)?;
    len.write(buffer);
    // packet identifier
    buffer.put_u16(packet.pkid);

    // topic filters
    for f in packet.filters.iter() {
        f.write(buffer);
    }

    Ok(1 + len.length() + len.value())
}

pub fn len(packet: &Subscribe) -> Result<VarInt, Error> {
    // len of pkid + vec![subscribe filter len]
    let len = 2 + packet.filters.iter().fold(0, |s, t| s + t.len());
    VarInt::new(len)
}

#[cfg(test)]
mod test {
    use bytes::BytesMut;
    use pretty_assertions::assert_eq;

    use super::*;
    use crate::packet::V4;
    use crate::{Packet, Properties, Protocol, QoS};

    #[test]
    fn subscribe_parsing_works() {
        let stream = &[
            0b1000_0010,
            20, // packet type, flags and remaining len
            0x01,
            0x04, // variable header. pkid = 260
            0x00,
            0x03,
            b'a',
            b'/',
            b'+', // payload. topic filter = 'a/+'
            0x00, // payload. qos = 0
            0x00,
            0x01,
            b'#', // payload. topic filter = '#'
            0x01, // payload. qos = 1
            0x00,
            0x05,
            b'a',
            b'/',
            b'b',
            b'/',
            b'c', // payload. topic filter = 'a/b/c'
            0x02, // payload. qos = 2
            0xDE,
            0xAD,
            0xBE,
            0xEF, // extra packets in the stream
        ];
        let mut stream = BytesMut::from(&stream[..]);
        let packet = V4::read(&mut stream, 128).unwrap();

        assert_eq!(
            packet,
            Packet::Subscribe(Subscribe {
                pkid: 260,
                filters: vec![
                    Filter::new("a/+".to_owned(), QoS::AtMostOnce),
                    Filter::new("#".to_owned(), QoS::AtLeastOnce),
                    Filter::new("a/b/c".to_owned(), QoS::ExactlyOnce)
                ],
                properties: Properties::new(),
            })
        );
    }

    #[test]
    fn subscribe_encoding_works() {
        let subscribe = Subscribe {
            pkid: 260,
            filters: vec![
                Filter::new("a/+".to_owned(), QoS::AtMostOnce),
                Filter::new("#".to_owned(), QoS::AtLeastOnce),
                Filter::new("a/b/c".to_owned(), QoS::ExactlyOnce),
            ],
            properties: Properties::new(),
        };

        let mut buf = BytesMut::new();
        write(&subscribe, &mut buf).unwrap();
        assert_eq!(
            buf,
            vec![
                0b1000_0010,
                20,
                0x01,
                0x04, // pkid = 260
                0x00,
                0x03,
                b'a',
                b'/',
                b'+', // topic filter = 'a/+'
                0x00, // qos = 0
                0x00,
                0x01,
                b'#', // topic filter = '#'
                0x01, // qos = 1
                0x00,
                0x05,
                b'a',
                b'/',
                b'b',
                b'/',
                b'c', // topic filter = 'a/b/c'
                0x02  // qos = 2
            ]
        );
    }
}
