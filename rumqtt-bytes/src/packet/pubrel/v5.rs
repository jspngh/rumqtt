use bytes::{BufMut, Bytes, BytesMut};

use super::{PubRel, PubRelProperties, PubRelReasonCode};
use crate::{parse::*, Error, FixedHeader};

pub fn read(fixed_header: FixedHeader, mut bytes: Bytes) -> Result<PubRel, Error> {
    let pkid = read_u16(&mut bytes)?;
    if fixed_header.remaining_len == 2 {
        return Ok(PubRel::new(pkid));
    }

    let ack_reason = read_u8(&mut bytes)?;
    if fixed_header.remaining_len < 4 {
        // Properties length is omitted
        return Ok(PubRel {
            pkid,
            reason: ack_reason.try_into()?,
            properties: None,
        });
    }

    Ok(PubRel {
        pkid,
        reason: ack_reason.try_into()?,
        properties: PubRelProperties::read(&mut bytes)?,
    })
}

pub fn write(packet: &PubRel, buffer: &mut BytesMut) -> Result<usize, Error> {
    // packet type and flags
    buffer.put_u8(0x62);
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

pub fn len(packet: &PubRel) -> Result<VarInt, Error> {
    let mut len = 2; // packet identifier

    if packet.reason == PubRelReasonCode::Success && packet.properties.is_none() {
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
        let pubrel_props = PubRelProperties {
            reason_string: None,
            user_properties: vec![(USER_PROP_KEY.into(), USER_PROP_VAL.into())],
        };

        let pubrel_pkt = PubRel {
            pkid: 1,
            reason: PubRelReasonCode::Success,
            properties: Some(pubrel_props),
        };

        let size_from_size = size_from_len(len(&pubrel_pkt).unwrap());
        let size_from_write = write(&pubrel_pkt, &mut dummy_bytes).unwrap();
        let size_from_bytes = dummy_bytes.len();

        assert_eq!(size_from_write, size_from_bytes);
        assert_eq!(size_from_size, size_from_bytes);
    }
}
