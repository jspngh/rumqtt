use bytes::{BufMut, Bytes, BytesMut};

use super::{PubComp, PubCompReasonCode};
use crate::{
    parse::*,
    property::{Properties, PropertyType},
    Error, FixedHeader,
};

const ALLOWED_PROPERTIES: &[PropertyType] =
    &[PropertyType::ReasonString, PropertyType::UserProperty];

pub fn read(fixed_header: FixedHeader, mut bytes: Bytes) -> Result<PubComp, Error> {
    let pkid = read_u16(&mut bytes)?;

    if fixed_header.remaining_len == 2 {
        return Ok(PubComp::new(pkid));
    }

    let ack_reason = read_u8(&mut bytes)?;
    if fixed_header.remaining_len < 4 {
        // Properties length is omitted
        return Ok(PubComp {
            pkid,
            reason: ack_reason.try_into()?,
            properties: Properties::new(),
        });
    }

    Ok(PubComp {
        pkid,
        reason: ack_reason.try_into()?,
        properties: Properties::read(&mut bytes, ALLOWED_PROPERTIES)?,
    })
}

pub fn write(packet: &PubComp, buffer: &mut BytesMut) -> Result<usize, Error> {
    // packet type and flags
    buffer.put_u8(0x70);
    // remaining length
    let len = len(packet)?;
    len.write(buffer);
    // packet identifier
    buffer.put_u16(packet.pkid);

    if len > 2 {
        // reason code
        buffer.put_u8(packet.reason as u8);
        // properties
        packet.properties.write(buffer)?;
    }

    Ok(1 + len.length() + len.value())
}

pub fn len(packet: &PubComp) -> Result<VarInt, Error> {
    let mut len = 2; // packet identifier

    if packet.reason == PubCompReasonCode::Success && packet.properties.is_empty() {
        // Reason code and property length can be omitted in this case
        return VarInt::new(len);
    }

    len += 1; // reason code

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

        let pubcomp_pkt = PubComp {
            pkid: 1,
            reason: PubCompReasonCode::Success,
            properties,
        };

        let size_from_size = size_from_len(len(&pubcomp_pkt).unwrap());
        let size_from_write = write(&pubcomp_pkt, &mut dummy_bytes).unwrap();
        let size_from_bytes = dummy_bytes.len();

        assert_eq!(size_from_write, size_from_bytes);
        assert_eq!(size_from_size, size_from_bytes);
    }
}
