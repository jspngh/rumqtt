//! MQTT protocol version 4 implementation

use bytes::BytesMut;

use super::*;
use crate::{Error, FixedHeader, Protocol};

/// Type that implements [`Packet`] reading and writing for MQTT protocol version 4 (MQTT v3.1.1)
#[derive(Copy, Clone)]
pub struct V4;

impl Protocol for V4 {
    type Item = Packet;

    fn read(stream: &mut BytesMut, max_size: u32) -> Result<Self::Item, Error> {
        let fixed_header = FixedHeader::check(stream.iter(), max_size)?;
        let packet_type = fixed_header.packet_type()?;

        // Test with a stream with exactly the size to check border panics
        let mut packet = stream.split_to(fixed_header.packet_size());
        // skip the fixed header, we have already parsed it
        let _ = packet.split_to(fixed_header.size());

        if fixed_header.remaining_len == 0 {
            // no payload packets
            match packet_type {
                PacketType::Disconnect | PacketType::PingReq | PacketType::PingResp => (),
                _ => return Err(Error::PayloadRequired),
            };
        }

        let packet = packet.freeze();
        let packet = match packet_type {
            PacketType::Connect => Packet::Connect(connect::v4::read(fixed_header, packet)?),
            PacketType::ConnAck => Packet::ConnAck(connack::v4::read(fixed_header, packet)?),
            PacketType::Publish => Packet::Publish(publish::v4::read(fixed_header, packet)?),
            PacketType::PubAck => Packet::PubAck(puback::v4::read(fixed_header, packet)?),
            PacketType::PubRec => Packet::PubRec(pubrec::v4::read(fixed_header, packet)?),
            PacketType::PubRel => Packet::PubRel(pubrel::v4::read(fixed_header, packet)?),
            PacketType::PubComp => Packet::PubComp(pubcomp::v4::read(fixed_header, packet)?),
            PacketType::Subscribe => Packet::Subscribe(subscribe::v4::read(fixed_header, packet)?),
            PacketType::SubAck => Packet::SubAck(suback::v4::read(fixed_header, packet)?),
            PacketType::Unsubscribe => {
                Packet::Unsubscribe(unsubscribe::v4::read(fixed_header, packet)?)
            }
            PacketType::UnsubAck => Packet::UnsubAck(unsuback::v4::read(fixed_header, packet)?),
            PacketType::PingReq => Packet::PingReq(ping::req::read(fixed_header, packet)?),
            PacketType::PingResp => Packet::PingResp(ping::resp::read(fixed_header, packet)?),
            PacketType::Disconnect => {
                Packet::Disconnect(disconnect::v4::read(fixed_header, packet)?)
            }
            p => return Err(Error::InvalidPacketType(p as u8)),
        };

        Ok(packet)
    }

    fn write(packet: Self::Item, stream: &mut BytesMut, max_size: u32) -> Result<usize, Error> {
        let size = size(&packet)?;
        if size > max_size {
            return Err(Error::OutgoingPacketTooLarge {
                pkt_size: size,
                max: max_size,
            });
        }

        match packet {
            Packet::Connect(c) => connect::v4::write(&c, stream),
            Packet::ConnAck(c) => connack::v4::write(&c, stream),
            Packet::Publish(p) => publish::v4::write(&p, stream),
            Packet::PubAck(p) => puback::v4::write(&p, stream),
            Packet::PubRec(p) => pubrec::v4::write(&p, stream),
            Packet::PubRel(p) => pubrel::v4::write(&p, stream),
            Packet::PubComp(p) => pubcomp::v4::write(&p, stream),
            Packet::Subscribe(s) => subscribe::v4::write(&s, stream),
            Packet::SubAck(s) => suback::v4::write(&s, stream),
            Packet::Unsubscribe(u) => unsubscribe::v4::write(&u, stream),
            Packet::UnsubAck(u) => unsuback::v4::write(&u, stream),
            Packet::PingReq(p) => ping::req::write(&p, stream),
            Packet::PingResp(p) => ping::resp::write(&p, stream),
            Packet::Disconnect(d) => disconnect::v4::write(&d, stream),
            Packet::Auth(_) => Err(Error::InvalidPacketType(15)),
        }
    }
}

fn size(packet: &Packet) -> Result<u32, Error> {
    let len = match packet {
        Packet::Connect(c) => connect::v4::len(c),
        Packet::ConnAck(c) => connack::v4::len(c),
        Packet::Publish(p) => publish::v4::len(p),
        Packet::PubAck(p) => puback::v4::len(p),
        Packet::PubRec(p) => pubrec::v4::len(p),
        Packet::PubRel(p) => pubrel::v4::len(p),
        Packet::PubComp(p) => pubcomp::v4::len(p),
        Packet::Subscribe(s) => subscribe::v4::len(s),
        Packet::SubAck(s) => suback::v4::len(s),
        Packet::Unsubscribe(u) => unsubscribe::v4::len(u),
        Packet::UnsubAck(u) => unsuback::v4::len(u),
        Packet::PingReq(p) => ping::req::len(p),
        Packet::PingResp(p) => ping::resp::len(p),
        Packet::Disconnect(d) => disconnect::v4::len(d),
        Packet::Auth(_) => Err(Error::InvalidPacketType(15)),
    }?;
    Ok(super::size_from_len(len) as u32)
}
