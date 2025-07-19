use bytes::{BufMut, Bytes, BytesMut};

use super::{Filter, Subscribe};
use crate::{
    parse::*,
    property::{Properties, PropertyType},
    Error, FixedHeader,
};

const ALLOWED_PROPERTIES: &[PropertyType] = &[
    PropertyType::SubscriptionIdentifier,
    PropertyType::UserProperty,
];

pub fn read(_fixed_header: FixedHeader, mut bytes: Bytes) -> Result<Subscribe, Error> {
    let pkid = read_u16(&mut bytes)?;
    let properties = Properties::read(&mut bytes, ALLOWED_PROPERTIES)?;

    let filters = Filter::read(&mut bytes)?;

    match filters.len() {
        0 => Err(Error::EmptySubscription),
        _ => Ok(Subscribe {
            pkid,
            filters,
            properties,
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
    // properties
    packet.properties.write(buffer)?;

    // topic filters
    for f in packet.filters.iter() {
        f.write(buffer);
    }

    Ok(1 + len.length() + len.value())
}

pub fn len(packet: &Subscribe) -> Result<VarInt, Error> {
    let mut len = 2 + packet.filters.iter().fold(0, |s, t| s + t.len());

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
    use crate::{properties, Property, QoS};

    #[test]
    fn length_calculation() {
        let mut dummy_bytes = BytesMut::new();
        // Use user_properties to pad the size to exceed ~128 bytes to make the
        // remaining_length field in the packet be 2 bytes long.
        let properties = properties![Property::UserProperty {
            name: USER_PROP_KEY.into(),
            value: USER_PROP_VAL.into(),
        }];

        let subscribe_pkt = Subscribe::new(
            Filter::new("hello/world".to_owned(), QoS::AtMostOnce),
            Some(properties),
        );

        let size_from_size = size_from_len(len(&subscribe_pkt).unwrap());
        let size_from_write = write(&subscribe_pkt, &mut dummy_bytes).unwrap();
        let size_from_bytes = dummy_bytes.len();

        assert_eq!(size_from_write, size_from_bytes);
        assert_eq!(size_from_size, size_from_bytes);
    }
}
