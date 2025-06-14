use bytes::{Buf, BufMut, Bytes, BytesMut};

use super::{UnsubAck, UnsubAckProperties};
use crate::{parse::*, Error, FixedHeader};

pub fn read(_fixed_header: FixedHeader, mut bytes: Bytes) -> Result<UnsubAck, Error> {
    let pkid = read_u16(&mut bytes)?;
    let properties = UnsubAckProperties::read(&mut bytes)?;

    if !bytes.has_remaining() {
        return Err(Error::MalformedPacket);
    }

    let mut reason_codes = Vec::new();
    while bytes.has_remaining() {
        let r = read_u8(&mut bytes)?;
        reason_codes.push(r.try_into()?);
    }

    Ok(UnsubAck {
        pkid,
        reason_codes,
        properties,
    })
}

pub fn write(packet: &UnsubAck, buffer: &mut BytesMut) -> Result<usize, Error> {
    // packet type and flags
    buffer.put_u8(0xB0);
    // remaining length
    let len = len(packet)?;
    len.write(buffer);
    // packet identifier
    buffer.put_u16(packet.pkid);

    // properties
    if let Some(p) = &packet.properties {
        p.write(buffer)?;
    } else {
        buffer.put_u8(0);
    }

    // reason codes
    let p = packet.reason_codes.iter().map(|&c| c as u8);
    buffer.extend(p);

    Ok(1 + len.length() + len.value())
}

pub fn len(packet: &UnsubAck) -> Result<VarInt, Error> {
    let mut len = 2 + packet.reason_codes.len();

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
        UnsubscribeReasonCode,
    };

    #[test]
    fn length_calculation() {
        let mut dummy_bytes = BytesMut::new();
        // Use user_properties to pad the size to exceed ~128 bytes to make the
        // remaining_length field in the packet be 2 bytes long.
        let unsuback_props = UnsubAckProperties {
            reason_string: None,
            user_properties: vec![(USER_PROP_KEY.into(), USER_PROP_VAL.into())],
        };

        let unsuback_pkt = UnsubAck {
            pkid: 1,
            reason_codes: vec![UnsubscribeReasonCode::Success],
            properties: Some(unsuback_props),
        };

        let size_from_size = size_from_len(len(&unsuback_pkt).unwrap());
        let size_from_write = write(&unsuback_pkt, &mut dummy_bytes).unwrap();
        let size_from_bytes = dummy_bytes.len();

        assert_eq!(size_from_write, size_from_bytes);
        assert_eq!(size_from_size, size_from_bytes);
    }
}
