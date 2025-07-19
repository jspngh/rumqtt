use bytes::{BufMut, Bytes, BytesMut};

use super::{ConnAck, ConnectReasonCode};
use crate::parse::*;
use crate::property::{Properties, PropertyType};
use crate::{Error, FixedHeader};

const ALLOWED_PROPERTIES: &[PropertyType] = &[
    PropertyType::SessionExpiryInterval,
    PropertyType::ReceiveMaximum,
    PropertyType::MaximumQos,
    PropertyType::RetainAvailable,
    PropertyType::MaximumPacketSize,
    PropertyType::AssignedClientIdentifier,
    PropertyType::TopicAliasMaximum,
    PropertyType::ReasonString,
    PropertyType::UserProperty,
    PropertyType::WildcardSubscriptionAvailable,
    PropertyType::SubscriptionIdentifierAvailable,
    PropertyType::SharedSubscriptionAvailable,
    PropertyType::ServerKeepAlive,
    PropertyType::ResponseInformation,
    PropertyType::ServerReference,
    PropertyType::AuthenticationMethod,
    PropertyType::AuthenticationData,
];

pub fn read(_fixed_header: FixedHeader, mut bytes: Bytes) -> Result<ConnAck, Error> {
    let flags = read_u8(&mut bytes)?;
    let return_code = read_u8(&mut bytes)?;
    let properties = Properties::read(&mut bytes, ALLOWED_PROPERTIES)?;

    let session_present = (flags & 0x01) == 1;
    let code = ConnectReasonCode::try_from(return_code)?;
    Ok(ConnAck {
        session_present,
        code,
        properties,
    })
}

pub fn write(packet: &ConnAck, buffer: &mut BytesMut) -> Result<usize, Error> {
    // packet type and flags
    buffer.put_u8(0x20);
    // remaining length
    let len = len(packet)?;
    len.write(buffer);
    // connect acknowledge flags
    buffer.put_u8(packet.session_present as u8);
    // reason code
    buffer.put_u8(packet.code as u8);
    // properties
    packet.properties.write(buffer)?;

    Ok(1 + len.length() + len.value())
}

pub fn len(packet: &ConnAck) -> Result<VarInt, Error> {
    let mut len = 1  // connect acknowledge flags
                + 1; // connect reason code

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
        let connack_props = properties![Property::UserProperty {
            name: USER_PROP_KEY.into(),
            value: USER_PROP_VAL.into(),
        }];

        let connack_pkt = ConnAck {
            session_present: false,
            code: ConnectReasonCode::Success,
            properties: connack_props,
        };

        let size_from_size = size_from_len(len(&connack_pkt).unwrap());
        let size_from_write = write(&connack_pkt, &mut dummy_bytes).unwrap();
        let size_from_bytes = dummy_bytes.len();

        assert_eq!(size_from_write, size_from_bytes);
        assert_eq!(size_from_size, size_from_bytes);
    }
}
