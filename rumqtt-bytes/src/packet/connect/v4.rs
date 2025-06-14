use bytes::{BufMut, Bytes, BytesMut};

use super::{Connect, LastWill, Login};
use crate::parse::*;
use crate::{Error, FixedHeader, QoS};

pub fn read(_fixed_header: FixedHeader, mut bytes: Bytes) -> Result<Connect, Error> {
    let protocol_name = read_mqtt_string(&mut bytes)?;
    if protocol_name != "MQTT" {
        return Err(Error::InvalidProtocol);
    }

    let protocol_level = read_u8(&mut bytes)?;
    if protocol_level != 4 {
        return Err(Error::InvalidProtocolLevel(protocol_level));
    }

    let connect_flags = read_u8(&mut bytes)?;
    let clean_session = (connect_flags & 0b10) != 0;
    let keep_alive = read_u16(&mut bytes)?;

    let client_id = read_mqtt_string(&mut bytes)?;
    let last_will = will::read(connect_flags, &mut bytes)?;
    let login = Login::read(connect_flags, &mut bytes)?;

    Ok(Connect {
        keep_alive,
        clean_start: clean_session,
        properties: None,
        client_id,
        last_will,
        login,
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
    buffer.put_u8(0x04);
    // connect flags
    let flags_index = 1 + len.length() + 2 + 4 + 1;
    let mut connect_flags = if packet.clean_start { 0b10 } else { 0 };
    buffer.put_u8(connect_flags);
    // keep alive time
    buffer.put_u16(packet.keep_alive);

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
    buffer[flags_index] = connect_flags;
    Ok(1 + len.length() + len.value())
}

pub fn len(packet: &Connect) -> Result<VarInt, Error> {
    let mut len = 6 // protocol name
                + 1  // protocol version
                + 1  // connect flags
                + 2; // keep alive

    len += 2 + packet.client_id.len();

    // last will len
    if let Some(w) = &packet.last_will {
        len += will::len(w);
    }

    // username and password len
    if let Some(l) = &packet.login {
        len += l.len();
    }

    VarInt::new(len)
}

mod will {
    use super::*;

    impl LastWill {
        pub fn new(
            topic: impl Into<String>,
            payload: impl Into<Vec<u8>>,
            qos: QoS,
            retain: bool,
        ) -> LastWill {
            LastWill {
                topic: topic.into(),
                payload: Bytes::from(payload.into()),
                qos,
                retain,
                properties: None,
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
                let topic = read_mqtt_string(bytes)?;
                let payload = read_mqtt_bytes(bytes)?;
                let qos = QoS::try_from((connect_flags & 0b11000) >> 3)?;
                let retain = (connect_flags & 0b0010_0000) != 0;
                Some(LastWill {
                    topic,
                    payload,
                    qos,
                    retain,
                    properties: None,
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

        write_mqtt_string(buffer, &packet.topic);
        write_mqtt_bytes(buffer, &packet.payload);
        Ok(connect_flags)
    }

    pub fn len(packet: &LastWill) -> usize {
        2 + packet.topic.len() + 2 + packet.payload.len()
    }
}
