use bytes::{BufMut, Bytes, BytesMut};

use super::{Disconnect, DisconnectReasonCode};
use crate::{
    parse::*,
    property::{Properties, PropertyType},
    Error, FixedHeader,
};

const ALLOWED_PROPERTIES: &[PropertyType] = &[
    PropertyType::SessionExpiryInterval,
    PropertyType::ReasonString,
    PropertyType::UserProperty,
    PropertyType::ServerReference,
];

pub fn read(fixed_header: FixedHeader, mut bytes: Bytes) -> Result<Disconnect, Error> {
    if fixed_header.flags() != 0x00 {
        return Err(Error::MalformedPacket);
    };

    if fixed_header.remaining_len == 0 {
        return Ok(Disconnect::new());
    }

    let reason_code = read_u8(&mut bytes)?;
    if fixed_header.remaining_len < 2 {
        // Property length is omitted, no properties
        return Ok(Disconnect {
            reason_code: reason_code.try_into()?,
            properties: Properties::new(),
        });
    }

    Ok(Disconnect {
        reason_code: reason_code.try_into()?,
        properties: Properties::read(&mut bytes, ALLOWED_PROPERTIES)?,
    })
}

pub fn write(packet: &Disconnect, buffer: &mut BytesMut) -> Result<usize, Error> {
    // packet type and flags
    buffer.put_u8(0xE0);
    // remaining length
    let len = len(packet)?;
    len.write(buffer);

    if len > 0 {
        // reason code
        buffer.put_u8(packet.reason_code as u8);
        // properties
        packet.properties.write(buffer)?;
    }

    Ok(1 + len.length() + len.value())
}

pub fn len(packet: &Disconnect) -> Result<VarInt, Error> {
    if packet.reason_code == DisconnectReasonCode::NormalDisconnection
        && packet.properties.is_empty()
    {
        return VarInt::new(0); // No variable header
    }

    let mut len = 1; // reason code

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
        v5::V5,
    };
    use crate::{properties, Packet, Property, Protocol};

    #[test]
    fn disconnect1_parsing_works() {
        let mut buffer = bytes::BytesMut::new();
        let packet_bytes = [
            0xE0, // Packet type
            0x00, // Remaining length
        ];
        let expected = Packet::Disconnect(Disconnect::new());

        buffer.extend_from_slice(&packet_bytes[..]);
        let packet = V5::read(&mut buffer, 128).unwrap();

        assert_eq!(packet, expected);
    }

    #[test]
    fn disconnect1_encoding_works() {
        let mut buffer = BytesMut::new();
        let disconnect = Disconnect::new();
        let expected = [
            0xE0, // Packet type
            0x00, // Remaining length
        ];

        write(&disconnect, &mut buffer).unwrap();

        assert_eq!(&buffer[..], &expected);
    }

    fn sample2() -> Disconnect {
        let properties = properties![
            Property::SessionExpiryInterval(1234), // TODO: change to 2137 xD
            Property::ReasonString("test".to_owned()),
            Property::UserProperty {
                name: "test".to_owned(),
                value: "test".to_owned(),
            },
            Property::ServerReference("test".to_owned()),
        ];

        Disconnect {
            reason_code: DisconnectReasonCode::UnspecifiedError,
            properties,
        }
    }

    fn sample_bytes2() -> Vec<u8> {
        vec![
            0xE0, // Packet type
            0x22, // Remaining length
            0x80, // Disconnect Reason Code
            0x20, // Properties length
            0x11, 0x00, 0x00, 0x04, 0xd2, // Session expiry interval
            0x1F, 0x00, 0x04, 0x74, 0x65, 0x73, 0x74, // Reason string
            0x26, 0x00, 0x04, 0x74, 0x65, 0x73, 0x74, 0x00, 0x04, 0x74, 0x65, 0x73,
            0x74, // User properties
            0x1C, 0x00, 0x04, 0x74, 0x65, 0x73, 0x74, // server reference
        ]
    }

    #[test]
    fn disconnect2_parsing_works() {
        let mut buffer = bytes::BytesMut::new();
        let packet_bytes = sample_bytes2();
        let expected = Packet::Disconnect(sample2());

        buffer.extend_from_slice(&packet_bytes[..]);
        let disconnect = V5::read(&mut buffer, 128).unwrap();

        assert_eq!(disconnect, expected);
    }

    #[test]
    fn disconnect2_encoding_works() {
        let mut buffer = BytesMut::new();

        let disconnect = sample2();
        let expected = sample_bytes2();

        write(&disconnect, &mut buffer).unwrap();

        assert_eq!(&buffer[..], &expected);
    }

    #[test]
    fn length_calculation() {
        let mut dummy_bytes = BytesMut::new();
        // Use user_properties to pad the size to exceed ~128 bytes to make the
        // remaining_length field in the packet be 2 bytes long.
        let properties = properties![Property::UserProperty {
            name: USER_PROP_KEY.to_owned(),
            value: USER_PROP_VAL.to_owned(),
        }];

        let mut disconn_pkt = Disconnect::new();
        disconn_pkt.properties = properties;

        let size_from_size = size_from_len(len(&disconn_pkt).unwrap());
        let size_from_write = write(&disconn_pkt, &mut dummy_bytes).unwrap();
        let size_from_bytes = dummy_bytes.len();

        assert_eq!(size_from_write, size_from_bytes);
        assert_eq!(size_from_size, size_from_bytes);
    }
}
