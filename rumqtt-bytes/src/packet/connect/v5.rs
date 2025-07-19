use bytes::{BufMut, Bytes, BytesMut};

use super::{Connect, Login};
use crate::parse::*;
use crate::property::{Properties, PropertyType};
use crate::{Error, FixedHeader};

const ALLOWED_PROPERTIES: &[PropertyType] = &[
    PropertyType::SessionExpiryInterval,
    PropertyType::ReceiveMaximum,
    PropertyType::MaximumPacketSize,
    PropertyType::TopicAliasMaximum,
    PropertyType::RequestResponseInformation,
    PropertyType::RequestProblemInformation,
    PropertyType::UserProperty,
    PropertyType::AuthenticationMethod,
    PropertyType::AuthenticationData,
];

pub fn read(_fixed_header: FixedHeader, mut bytes: Bytes) -> Result<Connect, Error> {
    let protocol_name = read_mqtt_bytes(&mut bytes)?;
    if protocol_name != "MQTT" {
        return Err(Error::InvalidProtocol);
    }

    let protocol_level = read_u8(&mut bytes)?;
    if protocol_level != 5 {
        return Err(Error::InvalidProtocolLevel(protocol_level));
    }

    let connect_flags = read_u8(&mut bytes)?;
    let clean_start = (connect_flags & 0b10) != 0;
    let keep_alive = read_u16(&mut bytes)?;

    let properties = Properties::read(&mut bytes, ALLOWED_PROPERTIES)?;

    let client_id = read_mqtt_string(&mut bytes)?;
    let last_will = will::read(connect_flags, &mut bytes)?;
    let login = Login::read(connect_flags, &mut bytes)?;

    Ok(Connect {
        keep_alive,
        clean_start,
        properties,
        client_id,
        last_will: last_will.map(Box::new),
        login: login.map(Box::new),
    })
}

pub fn write(packet: &Connect, buffer: &mut BytesMut) -> Result<usize, Error> {
    // packet type and flags
    buffer.put_u8(0x10);
    // remaining length
    let len = len(packet)?;
    len.write(buffer);
    // protocol name
    write_mqtt_string(buffer, "MQTT");
    // protocol version
    buffer.put_u8(0x05);
    // connect flags
    let connect_flags_index = 1 + len.length() + 2 + 4 + 1;
    let mut connect_flags = if packet.clean_start { 0b10 } else { 0 };
    buffer.put_u8(connect_flags);
    // keep alive time
    buffer.put_u16(packet.keep_alive);
    // properties
    packet.properties.write(buffer)?;

    // client identifier
    write_mqtt_string(buffer, &packet.client_id);

    // last will message
    if let Some(w) = &packet.last_will {
        connect_flags |= will::write(w, buffer)?;
    }

    // username and password
    if let Some(l) = &packet.login {
        connect_flags |= l.write(buffer);
    }

    // update connect flags
    buffer[connect_flags_index] = connect_flags;
    Ok(1 + len.length() + len.value())
}

pub fn len(packet: &Connect) -> Result<VarInt, Error> {
    let mut len = 6  // protocol name
                + 1  // protocol version
                + 1  // connect flags
                + 2; // keep alive

    let properties_len = packet.properties.len()?;
    len += properties_len.length() + properties_len.value();

    len += 2 + packet.client_id.len();

    // last will len
    if let Some(w) = &packet.last_will {
        len += will::len(w)?;
    }

    // username and password len
    if let Some(l) = &packet.login {
        len += l.len();
    }

    VarInt::new(len)
}

mod will {
    use bytes::{Bytes, BytesMut};

    use crate::{
        packet::LastWill,
        parse::*,
        property::{Properties, PropertyType},
        Error, QoS,
    };

    const ALLOWED_PROPERTIES: &[PropertyType] = &[
        PropertyType::WillDelayInterval,
        PropertyType::PayloadFormatIndicator,
        PropertyType::MessageExpiryInterval,
        PropertyType::ContentType,
        PropertyType::ResponseTopic,
        PropertyType::CorrelationData,
        PropertyType::UserProperty,
    ];

    impl LastWill {
        pub fn new_v5(
            topic: impl Into<String>,
            payload: impl Into<Vec<u8>>,
            qos: QoS,
            retain: bool,
        ) -> Self {
            Self {
                topic: topic.into(),
                payload: Bytes::from(payload.into()),
                qos,
                retain,
                properties: Properties::new(),
            }
        }
    }

    pub fn read(connect_flags: u8, bytes: &mut Bytes) -> Result<Option<LastWill>, Error> {
        let last_will = match connect_flags & 0b100 {
            0 if (connect_flags & 0b0011_1000) != 0 => {
                return Err(Error::IncorrectPacketFormat);
            }
            0 => None,
            _ => {
                // Properties in variable header
                let properties = Properties::read(bytes, ALLOWED_PROPERTIES)?;

                let topic = read_mqtt_string(bytes)?;
                let payload = read_mqtt_bytes(bytes)?;
                let qos = QoS::try_from((connect_flags & 0b11000) >> 3)?;
                let retain = (connect_flags & 0b0010_0000) != 0;
                Some(LastWill {
                    topic,
                    payload,
                    qos,
                    retain,
                    properties,
                })
            }
        };

        Ok(last_will)
    }

    pub fn write(packet: &LastWill, buffer: &mut BytesMut) -> Result<u8, Error> {
        let mut connect_flags = 0b100 | ((packet.qos as u8) << 3);
        if packet.retain {
            connect_flags |= 0x20;
        }

        packet.properties.write(buffer)?;

        write_mqtt_string(buffer, &packet.topic);
        write_mqtt_bytes(buffer, &packet.payload);
        Ok(connect_flags)
    }

    pub fn len(packet: &LastWill) -> Result<usize, Error> {
        let mut len = 2 + packet.topic.len() + 2 + packet.payload.len();

        let properties_len = packet.properties.len()?;
        len += properties_len.length() + properties_len.value();

        Ok(len)
    }
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
        let connect_props = properties![Property::UserProperty {
            name: USER_PROP_KEY.into(),
            value: USER_PROP_VAL.into(),
        }];
        let connect_pkt = Connect {
            keep_alive: 5,
            client_id: "client".into(),
            clean_start: true,
            properties: connect_props,
            last_will: None,
            login: None,
        };

        let size_from_size = size_from_len(len(&connect_pkt).unwrap());
        let size_from_write = write(&connect_pkt, &mut dummy_bytes).unwrap();
        let size_from_bytes = dummy_bytes.len();

        assert_eq!(size_from_write, size_from_bytes);
        assert_eq!(size_from_size, size_from_bytes);
    }
}
