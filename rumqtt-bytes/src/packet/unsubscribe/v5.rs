use bytes::{Buf, BufMut, Bytes, BytesMut};

use super::Unsubscribe;
use crate::{
    parse::*,
    property::{Properties, PropertyType},
    Error, FixedHeader,
};

const ALLOWED_PROPERTIES: &[PropertyType] = &[PropertyType::UserProperty];

pub fn read(_fixed_header: FixedHeader, mut bytes: Bytes) -> Result<Unsubscribe, Error> {
    let pkid = read_u16(&mut bytes)?;
    let properties = Properties::read(&mut bytes, ALLOWED_PROPERTIES)?;

    if !bytes.has_remaining() {
        return Err(Error::MalformedPacket);
    }

    let mut filters = Vec::new();
    while bytes.has_remaining() {
        let filter = read_mqtt_string(&mut bytes)?;
        filters.push(filter);
    }

    Ok(Unsubscribe {
        pkid,
        filters,
        properties,
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
    // properties
    packet.properties.write(buffer)?;

    // topic filters
    for filter in packet.filters.iter() {
        write_mqtt_string(buffer, filter);
    }

    Ok(1 + len.length() + len.value())
}

pub fn len(packet: &Unsubscribe) -> Result<VarInt, Error> {
    // Packet id + length of filters (unlike subscribe this just a string,
    // hence 2 is prefixed for len per filter)
    let mut len = 2 + packet.filters.iter().fold(0, |s, t| s + 2 + t.len());

    let properties_len = packet.properties.len()?;
    len += properties_len.length() + properties_len.value();

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
        let properties = properties![Property::UserProperty {
            name: USER_PROP_KEY.into(),
            value: USER_PROP_VAL.into(),
        }];

        let unsubscribe_pkt = Unsubscribe {
            pkid: 1,
            filters: vec!["hello/world".into()],
            properties,
        };

        let size_from_size = size_from_len(len(&unsubscribe_pkt).unwrap());
        let size_from_write = write(&unsubscribe_pkt, &mut dummy_bytes).unwrap();
        let size_from_bytes = dummy_bytes.len();

        assert_eq!(size_from_write, size_from_bytes);
        assert_eq!(size_from_size, size_from_bytes);
    }
}
