use bytes::{BufMut, Bytes, BytesMut};

use super::{Publish, PublishProperties};
use crate::{parse::*, Error, FixedHeader, QoS};

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

    let properties = PublishProperties::read(&mut bytes)?;
    Ok(Publish {
        dup,
        qos,
        retain,
        pkid,
        topic,
        properties,
        payload: bytes,
    })
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

    // packet identifier
    if packet.qos != QoS::AtMostOnce {
        let pkid = packet.pkid;
        if pkid == 0 {
            return Err(Error::PacketIdZero);
        }

        buffer.put_u16(pkid);
    }

    // properties
    if let Some(p) = &packet.properties {
        p.write(buffer)?;
    } else {
        buffer.put_u8(0);
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

    if let Some(p) = &packet.properties {
        let properties_len = p.len()?;
        len += properties_len.length() + properties_len.value();
    } else {
        len += 1; // 0 property length
    }

    len += packet.payload.len();
    VarInt::new(len)
}

#[cfg(test)]
mod test {
    use bytes::BytesMut;
    use pretty_assertions::assert_eq;

    use super::*;
    use crate::packet::{
        size_from_len,
        tests::{USER_PROP_KEY, USER_PROP_VAL},
    };

    #[test]
    fn length_calculation() {
        let mut dummy_bytes = BytesMut::new();
        // Use user_properties to pad the size to exceed ~128 bytes to make the
        // remaining_length field in the packet be 2 bytes long.
        let publish_props = PublishProperties {
            payload_format_indicator: None,
            message_expiry_interval: None,
            topic_alias: None,
            response_topic: None,
            correlation_data: None,
            user_properties: vec![(USER_PROP_KEY.into(), USER_PROP_VAL.into())],
            subscription_ids: vec![VarInt::new(1).unwrap()],
            content_type: None,
        };

        let mut publish_pkt = Publish::new("hello/world", QoS::AtMostOnce, vec![1; 10]);
        publish_pkt.properties = Some(publish_props);

        let size_from_size = size_from_len(len(&publish_pkt).unwrap());
        let size_from_write = write(&publish_pkt, &mut dummy_bytes).unwrap();
        let size_from_bytes = dummy_bytes.len();

        assert_eq!(size_from_write, size_from_bytes);
        assert_eq!(size_from_size, size_from_bytes);
    }
}
