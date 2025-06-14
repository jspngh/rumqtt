//! MQTT protocol serialization and deserialization
//!
//! This crate implements the assembling and disassembling of MQTT packets
//! for both version 4 of the protocol (MQTT 3.1.1) and version 5 (MQTT 5.0).
//!
//! It is mainly intended to be used with the rumqtt client and server.

use bytes::BytesMut;

mod codec;
mod error;
mod header;
mod packet;
mod parse;
mod property;
mod reason;

pub use codec::Codec;
pub use error::Error;
pub use packet::*;
pub use parse::VarInt;

use header::FixedHeader;

/// A type that can serialize and deserialize MQTT packets from/to a stream of bytes.
pub trait Protocol {
    /// The type that is being serialized and deserialized
    type Item;

    /// Deserializes a packet from a stream of bytes
    fn read(stream: &mut BytesMut, max_size: u32) -> Result<Self::Item, Error>;

    /// Serializes the packet into a stream of bytes
    fn write(packet: Self::Item, stream: &mut BytesMut, max_size: u32) -> Result<usize, Error>;
}

/// The supported MQTT protocol versions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProtocolVersion {
    /// MQTT 3.1.1
    V4,
    /// MQTT 5.0
    V5,
}

/// Quality of Service levels for packet delivery.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd)]
#[allow(clippy::enum_variant_names)]
pub enum QoS {
    AtMostOnce = 0,
    AtLeastOnce = 1,
    ExactlyOnce = 2,
}

impl Default for QoS {
    fn default() -> Self {
        Self::AtMostOnce
    }
}

impl TryFrom<u8> for QoS {
    type Error = Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(QoS::AtMostOnce),
            1 => Ok(QoS::AtLeastOnce),
            2 => Ok(QoS::ExactlyOnce),
            qos => Err(Error::InvalidQoS(qos)),
        }
    }
}
