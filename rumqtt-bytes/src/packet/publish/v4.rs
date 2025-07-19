use bytes::{BufMut, Bytes, BytesMut};

use super::Publish;
use crate::{parse::*, Error, FixedHeader, Properties, QoS};

pub fn read(fixed_header: FixedHeader, mut bytes: Bytes) -> Result<Publish, Error> {
    let dup = (fixed_header.flags() & 0b1000) != 0;
    let qos = QoS::try_from((fixed_header.flags() & 0b0110) >> 1)?;
    let retain = (fixed_header.flags() & 0b0001) != 0;

    let topic = read_mqtt_string(&mut bytes)?;

    // Packet identifier exists where QoS > 0
    let pkid = match qos {
        QoS::AtMostOnce => 0,
        QoS::AtLeastOnce | QoS::ExactlyOnce => read_u16(&mut bytes)?,
    };

    if qos != QoS::AtMostOnce && pkid == 0 {
        return Err(Error::PacketIdZero);
    }

    let publish = Publish {
        dup,
        retain,
        qos,
        pkid,
        topic,
        payload: bytes,
        properties: Properties::new(),
    };

    Ok(publish)
}

pub fn write(packet: &Publish, buffer: &mut BytesMut) -> Result<usize, Error> {
    // packet type and flags
    let dup = packet.dup as u8;
    let qos = packet.qos as u8;
    let retain = packet.retain as u8;
    buffer.put_u8(0b0011_0000 | retain | (qos << 1) | (dup << 3));
    // remaining length
    let len = len(packet)?;
    len.write(buffer);
    // topic
    write_mqtt_string(buffer, &packet.topic);

    if packet.qos != QoS::AtMostOnce {
        let pkid = packet.pkid;
        if pkid == 0 {
            return Err(Error::PacketIdZero);
        }

        buffer.put_u16(pkid);
    }

    buffer.extend_from_slice(&packet.payload);

    Ok(1 + len.length() + len.value())
}

pub fn len(packet: &Publish) -> Result<VarInt, Error> {
    let mut len = 2 + packet.topic.len();
    if packet.qos != QoS::AtMostOnce && packet.pkid != 0 {
        // packet identifier is only present for QoS > 0
        len += 2;
    }

    len += packet.payload.len();
    VarInt::new(len)
}

#[cfg(test)]
mod test {
    use bytes::{Bytes, BytesMut};
    use pretty_assertions::assert_eq;

    use super::*;
    use crate::packet::V4;
    use crate::{Packet, Protocol};

    #[test]
    fn qos1_publish_parsing_works() {
        let stream = &[
            0b0011_0010,
            11, // packet type, flags and remaining len
            0x00,
            0x03,
            b'a',
            b'/',
            b'b', // variable header. topic name = 'a/b'
            0x00,
            0x0a, // variable header. pkid = 10
            0xF1,
            0xF2,
            0xF3,
            0xF4, // publish payload
            0xDE,
            0xAD,
            0xBE,
            0xEF, // extra packets in the stream
        ];

        let mut stream = BytesMut::from(&stream[..]);
        let packet = V4::read(&mut stream, 128).unwrap();

        let payload = &[0xF1, 0xF2, 0xF3, 0xF4];
        assert_eq!(
            packet,
            Packet::Publish(Publish {
                dup: false,
                qos: QoS::AtLeastOnce,
                retain: false,
                topic: "a/b".to_owned(),
                pkid: 10,
                payload: Bytes::from(&payload[..]),
                properties: Properties::new(),
            })
        );
    }

    #[test]
    fn qos0_publish_parsing_works() {
        let stream = &[
            0b0011_0000,
            7, // packet type, flags and remaining len
            0x00,
            0x03,
            b'a',
            b'/',
            b'b', // variable header. topic name = 'a/b'
            0x01,
            0x02, // payload
            0xDE,
            0xAD,
            0xBE,
            0xEF, // extra packets in the stream
        ];

        let mut stream = BytesMut::from(&stream[..]);
        let packet = V4::read(&mut stream, 128).unwrap();

        assert_eq!(
            packet,
            Packet::Publish(Publish {
                dup: false,
                qos: QoS::AtMostOnce,
                retain: false,
                topic: "a/b".to_owned(),
                pkid: 0,
                payload: Bytes::from(&[0x01, 0x02][..]),
                properties: Properties::new(),
            })
        );
    }

    #[test]
    fn qos1_publish_encoding_works() {
        let publish = Publish {
            dup: false,
            qos: QoS::AtLeastOnce,
            retain: false,
            topic: "a/b".to_owned(),
            pkid: 10,
            payload: Bytes::from(vec![0xF1, 0xF2, 0xF3, 0xF4]),
            properties: Properties::new(),
        };

        let mut buf = BytesMut::new();
        write(&publish, &mut buf).unwrap();

        assert_eq!(
            buf,
            vec![
                0b0011_0010,
                11,
                0x00,
                0x03,
                b'a',
                b'/',
                b'b',
                0x00,
                0x0a,
                0xF1,
                0xF2,
                0xF3,
                0xF4
            ]
        );
    }

    #[test]
    fn qos0_publish_encoding_works() {
        let publish = Publish {
            dup: false,
            qos: QoS::AtMostOnce,
            retain: false,
            topic: "a/b".to_owned(),
            pkid: 0,
            payload: Bytes::from(vec![0xE1, 0xE2, 0xE3, 0xE4]),
            properties: Properties::new(),
        };

        let mut buf = BytesMut::new();
        write(&publish, &mut buf).unwrap();

        assert_eq!(
            buf,
            vec![
                0b0011_0000,
                9,
                0x00,
                0x03,
                b'a',
                b'/',
                b'b',
                0xE1,
                0xE2,
                0xE3,
                0xE4
            ]
        );
    }
}
