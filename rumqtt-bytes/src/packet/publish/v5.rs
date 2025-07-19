use bytes::{BufMut, Bytes, BytesMut};

use super::Publish;
use crate::property::{Properties, PropertyType};
use crate::{parse::*, Error, FixedHeader, QoS};

const ALLOWED_PROPERTIES: &[PropertyType] = &[
    PropertyType::PayloadFormatIndicator,
    PropertyType::MessageExpiryInterval,
    PropertyType::TopicAlias,
    PropertyType::ResponseTopic,
    PropertyType::CorrelationData,
    PropertyType::UserProperty,
    PropertyType::SubscriptionIdentifier,
    PropertyType::ContentType,
];

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

    let properties = Properties::read(&mut bytes, ALLOWED_PROPERTIES)?;
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
    packet.properties.write(buffer)?;

    buffer.extend_from_slice(&packet.payload);

    Ok(1 + len.length() + len.value())
}

pub fn len(packet: &Publish) -> Result<VarInt, Error> {
    let mut len = 2 + packet.topic.len();
    if packet.qos != QoS::AtMostOnce && packet.pkid != 0 {
        // packet identifier is only present for QoS > 0
        len += 2;
    }

    let properties_len = packet.properties.len()?;
    len += properties_len.length() + properties_len.value();

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
    use crate::{properties, Property};

    #[test]
    fn length_calculation() {
        let mut dummy_bytes = BytesMut::new();
        // Use user_properties to pad the size to exceed ~128 bytes to make the
        // remaining_length field in the packet be 2 bytes long.
        let publish_props = properties![
            Property::UserProperty {
                name: USER_PROP_KEY.into(),
                value: USER_PROP_VAL.into(),
            },
            Property::SubscriptionIdentifier(VarInt::new(1).unwrap()),
        ];

        let mut publish_pkt = Publish::new("hello/world", QoS::AtMostOnce, vec![1; 10]);
        publish_pkt.properties = publish_props;

        let size_from_size = size_from_len(len(&publish_pkt).unwrap());
        let size_from_write = write(&publish_pkt, &mut dummy_bytes).unwrap();
        let size_from_bytes = dummy_bytes.len();

        assert_eq!(size_from_write, size_from_bytes);
        assert_eq!(size_from_size, size_from_bytes);
    }
}
