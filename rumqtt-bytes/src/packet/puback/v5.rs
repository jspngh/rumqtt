use bytes::{BufMut, Bytes, BytesMut};

use super::{PubAck, PubAckProperties, PubAckReasonCode};
use crate::{parse::*, Error, FixedHeader};

pub fn read(fixed_header: FixedHeader, mut bytes: Bytes) -> Result<PubAck, Error> {
    let pkid = read_u16(&mut bytes)?;

    // No reason code or properties if remaining length == 2
    if fixed_header.remaining_len == 2 {
        return Ok(PubAck::new(pkid));
    }

    let ack_reason = read_u8(&mut bytes)?;
    if fixed_header.remaining_len < 4 {
        // Properties length is omitted
        return Ok(PubAck {
            pkid,
            reason: ack_reason.try_into()?,
            properties: None,
        });
    }

    Ok(PubAck {
        pkid,
        reason: ack_reason.try_into()?,
        properties: PubAckProperties::read(&mut bytes)?,
    })
}

pub fn write(packet: &PubAck, buffer: &mut BytesMut) -> Result<usize, Error> {
    // packet type and flags
    buffer.put_u8(0x40);
    // remaining length
    let len = len(packet)?;
    len.write(buffer);
    // packet identifier
    buffer.put_u16(packet.pkid);

    if len > 2 {
        // reason code
        buffer.put_u8(packet.reason as u8);
        // properties
        if let Some(p) = &packet.properties {
            p.write(buffer)?;
        } else {
            buffer.put_u8(0);
        }
    }

    Ok(1 + len.length() + len.value())
}

pub fn len(packet: &PubAck) -> Result<VarInt, Error> {
    let mut len = 2; // packet identifier

    if packet.reason == PubAckReasonCode::Success && packet.properties.is_none() {
        // Reason code and property length can be omitted in this case
        return VarInt::new(len);
    }

    len += 1; // reason code

    if let Some(p) = &packet.properties {
        let properties_len = p.len()?;
        len += properties_len.length() + properties_len.value();
    } else {
        len += 1; // 0 property length
    }

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
        let puback_props = PubAckProperties {
            reason_string: None,
            user_properties: vec![(USER_PROP_KEY.into(), USER_PROP_VAL.into())],
        };

        let puback_pkt = PubAck {
            pkid: 1,
            reason: PubAckReasonCode::Success,
            properties: Some(puback_props),
        };

        let size_from_size = size_from_len(len(&puback_pkt).unwrap());
        let size_from_write = write(&puback_pkt, &mut dummy_bytes).unwrap();
        let size_from_bytes = dummy_bytes.len();

        assert_eq!(size_from_write, size_from_bytes);
        assert_eq!(size_from_size, size_from_bytes);
    }
}
