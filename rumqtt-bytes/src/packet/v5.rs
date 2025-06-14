//! MQTT protocol version 5 implementation

use bytes::BytesMut;

use super::*;
use crate::{Error, FixedHeader, Protocol};

/// Type that implements [`Packet`] reading and writing for MQTT protocol version 5 (MQTT v5.0)
#[derive(Copy, Clone)]
pub struct V5;

impl Protocol for V5 {
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
            PacketType::Connect => Packet::Connect(connect::v5::read(fixed_header, packet)?),
            PacketType::ConnAck => Packet::ConnAck(connack::v5::read(fixed_header, packet)?),
            PacketType::Publish => Packet::Publish(publish::v5::read(fixed_header, packet)?),
            PacketType::PubAck => Packet::PubAck(puback::v5::read(fixed_header, packet)?),
            PacketType::PubRec => Packet::PubRec(pubrec::v5::read(fixed_header, packet)?),
            PacketType::PubRel => Packet::PubRel(pubrel::v5::read(fixed_header, packet)?),
            PacketType::PubComp => Packet::PubComp(pubcomp::v5::read(fixed_header, packet)?),
            PacketType::Subscribe => Packet::Subscribe(subscribe::v5::read(fixed_header, packet)?),
            PacketType::SubAck => Packet::SubAck(suback::v5::read(fixed_header, packet)?),
            PacketType::Unsubscribe => {
                Packet::Unsubscribe(unsubscribe::v5::read(fixed_header, packet)?)
            }
            PacketType::UnsubAck => Packet::UnsubAck(unsuback::v5::read(fixed_header, packet)?),
            PacketType::PingReq => Packet::PingReq(ping::req::read(fixed_header, packet)?),
            PacketType::PingResp => Packet::PingResp(ping::resp::read(fixed_header, packet)?),
            PacketType::Disconnect => {
                Packet::Disconnect(disconnect::v5::read(fixed_header, packet)?)
            }
            PacketType::Auth => Packet::Auth(auth::v5::read(fixed_header, packet)?),
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
            Packet::Connect(c) => connect::v5::write(&c, stream),
            Packet::ConnAck(c) => connack::v5::write(&c, stream),
            Packet::Publish(p) => publish::v5::write(&p, stream),
            Packet::PubAck(p) => puback::v5::write(&p, stream),
            Packet::PubRec(p) => pubrec::v5::write(&p, stream),
            Packet::PubRel(p) => pubrel::v5::write(&p, stream),
            Packet::PubComp(p) => pubcomp::v5::write(&p, stream),
            Packet::Subscribe(s) => subscribe::v5::write(&s, stream),
            Packet::SubAck(s) => suback::v5::write(&s, stream),
            Packet::Unsubscribe(u) => unsubscribe::v5::write(&u, stream),
            Packet::UnsubAck(u) => unsuback::v5::write(&u, stream),
            Packet::PingReq(p) => ping::req::write(&p, stream),
            Packet::PingResp(p) => ping::resp::write(&p, stream),
            Packet::Disconnect(d) => disconnect::v5::write(&d, stream),
            Packet::Auth(a) => auth::v5::write(&a, stream),
        }
    }
}

fn size(packet: &Packet) -> Result<u32, Error> {
    let len = match packet {
        Packet::Connect(c) => connect::v5::len(c),
        Packet::ConnAck(c) => connack::v5::len(c),
        Packet::Publish(p) => publish::v5::len(p),
        Packet::PubAck(p) => puback::v5::len(p),
        Packet::PubRec(p) => pubrec::v5::len(p),
        Packet::PubRel(p) => pubrel::v5::len(p),
        Packet::PubComp(p) => pubcomp::v5::len(p),
        Packet::Subscribe(s) => subscribe::v5::len(s),
        Packet::SubAck(s) => suback::v5::len(s),
        Packet::Unsubscribe(u) => unsubscribe::v5::len(u),
        Packet::UnsubAck(u) => unsuback::v5::len(u),
        Packet::PingReq(p) => ping::req::len(p),
        Packet::PingResp(p) => ping::resp::len(p),
        Packet::Disconnect(d) => disconnect::v5::len(d),
        Packet::Auth(a) => auth::v5::len(a),
    }?;
    Ok(super::size_from_len(len) as u32)
}
