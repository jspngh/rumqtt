use bytes::{Buf, BufMut, Bytes, BytesMut};

use super::{SubAck, SubAckProperties};
use crate::{parse::*, Error, FixedHeader};

pub fn read(_fixed_header: FixedHeader, mut bytes: Bytes) -> Result<SubAck, Error> {
    let pkid = read_u16(&mut bytes)?;
    let properties = SubAckProperties::read(&mut bytes)?;

    if !bytes.has_remaining() {
        return Err(Error::MalformedPacket);
    }

    let mut reason_codes = Vec::new();
    while bytes.has_remaining() {
        let return_code = read_u8(&mut bytes)?;
        reason_codes.push(return_code.try_into()?);
    }

    Ok(SubAck {
        pkid,
        properties,
        reason_codes,
    })
}

pub fn write(packet: &SubAck, buffer: &mut BytesMut) -> Result<usize, Error> {
    // packet type and flags
    buffer.put_u8(0x90);
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

    // return codes
    let p = packet.reason_codes.iter().map(|&c| u8::from(c));
    buffer.extend(p);

    Ok(1 + len.length() + len.value())
}

pub fn len(packet: &SubAck) -> Result<VarInt, Error> {
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
        SubscribeReasonCode,
    };
    use crate::QoS;

    #[test]
    fn length_calculation() {
        let mut dummy_bytes = BytesMut::new();
        // Use user_properties to pad the size to exceed ~128 bytes to make the
        // remaining_length field in the packet be 2 bytes long.
        let suback_props = SubAckProperties {
            reason_string: None,
            user_properties: vec![(USER_PROP_KEY.into(), USER_PROP_VAL.into())],
        };

        let suback_pkt = SubAck {
            pkid: 1,
            reason_codes: vec![SubscribeReasonCode::Success(QoS::ExactlyOnce)],
            properties: Some(suback_props),
        };

        let size_from_size = size_from_len(len(&suback_pkt).unwrap());
        let size_from_write = write(&suback_pkt, &mut dummy_bytes).unwrap();
        let size_from_bytes = dummy_bytes.len();

        assert_eq!(size_from_write, size_from_bytes);
        assert_eq!(size_from_size, size_from_bytes);
    }
}
