use bytes::{Buf, BufMut, Bytes, BytesMut};

use super::Unsubscribe;
use crate::{parse::*, Error, FixedHeader};

pub fn read(_fixed_header: FixedHeader, mut bytes: Bytes) -> Result<Unsubscribe, Error> {
    let pkid = read_u16(&mut bytes)?;

    if !bytes.has_remaining() {
        return Err(Error::MalformedPacket);
    }

    let mut filters = Vec::new();
    while bytes.has_remaining() {
        let topic_filter = read_mqtt_string(&mut bytes)?;
        filters.push(topic_filter);
    }

    Ok(Unsubscribe {
        pkid,
        filters,
        properties: None,
    })
}

pub fn write(packet: &Unsubscribe, buffer: &mut BytesMut) -> Result<usize, Error> {
    // packet type and flags
    buffer.put_u8(0xA2);
    // remaining length
    let len = len(packet)?;
    len.write(buffer);
    // packet identifier
    buffer.put_u16(packet.pkid);

    // topic filters
    for topic in packet.filters.iter() {
        write_mqtt_string(buffer, topic.as_str());
    }
    Ok(1 + len.length() + len.value())
}

pub fn len(packet: &Unsubscribe) -> Result<VarInt, Error> {
    // Packet id + length of filters
    let len = 2 + packet.filters.iter().fold(0, |s, t| s + 2 + t.len());
    VarInt::new(len)
}
