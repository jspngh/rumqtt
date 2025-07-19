//! This module defines the core MQTT packet types as specified by the MQTT protocol.
//!
//! At the heart of this module is the [`Packet`] enum, which consolidates all
//! supported MQTT control packets into a single type. This provides a unified
//! interface for handling different kinds of packets throughout the `rumqtt-bytes` crate
//! and downstream applications.
//!
//! ## Packet Structure
//!
//! Each variant of the `Packet` enum corresponds to a specific MQTT control packet,
//! such as `Connect`, `Publish`, `Subscribe`, and `Disconnect`.
//! These individual packet structs are defined in their respective submodules
//! and are re-exported here for convenience.
//!
//! The module supports both MQTT protocol v4 and v5, with version-specific details
//! handled internally.
//!
//! ## Usage
//!
//! This module is central to encoding and decoding MQTT packets.
//! When decoding a byte stream, the result is typically a `Packet` enum.
//! When encoding, you would construct the specific packet struct (e.g., `Publish`)
//! and then wrap it in the `Packet` enum before writing it to the network.
//!
//! This is done using the [`V4`] and [`V5`] structs, which implement the `Protocol` trait.

use crate::Error;

mod auth;
mod connack;
mod connect;
mod disconnect;
mod ping;
mod puback;
mod pubcomp;
mod publish;
mod pubrec;
mod pubrel;
mod suback;
mod subscribe;
mod unsuback;
mod unsubscribe;
mod v4;
mod v5;

pub use auth::{Auth, AuthReasonCode};
pub use connack::{ConnAck, ConnectReasonCode, ConnectReturnCode};
pub use connect::{Connect, LastWill, Login};
pub use disconnect::{Disconnect, DisconnectReasonCode};
pub use ping::{PingReq, PingResp};
pub use puback::{PubAck, PubAckReasonCode};
pub use pubcomp::{PubComp, PubCompReasonCode};
pub use publish::Publish;
pub use pubrec::{PubRec, PubRecReasonCode};
pub use pubrel::{PubRel, PubRelReasonCode};
pub use suback::{SubAck, SubscribeReasonCode};
pub use subscribe::{Filter, RetainForwardRule, Subscribe};
pub use unsuback::{UnsubAck, UnsubscribeReasonCode};
pub use unsubscribe::Unsubscribe;
pub use v4::V4;
pub use v5::V5;

/// MQTT Control Packet
///
/// This enumeration represents the different types of MQTT packets that can be sent or received.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Packet {
    Connect(Connect),
    ConnAck(ConnAck),
    Publish(Publish),
    PubAck(PubAck),
    PubRec(PubRec),
    PubRel(PubRel),
    PubComp(PubComp),
    Subscribe(Subscribe),
    SubAck(SubAck),
    Unsubscribe(Unsubscribe),
    UnsubAck(UnsubAck),
    PingReq(PingReq),
    PingResp(PingResp),
    Disconnect(Disconnect),
    Auth(Auth),
}

/// MQTT packet types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PacketType {
    /// Connection request
    Connect = 1,
    /// Connect acknowledgment
    ConnAck,
    /// Publish message
    Publish,
    /// Publish acknowledgment (QoS 1)
    PubAck,
    /// Publish received (QoS 2 delivery part 1)
    PubRec,
    /// Publish release (QoS 2 delivery part 2)
    PubRel,
    /// Publish complete (QoS 2 delivery part 3)
    PubComp,
    /// Subscribe request
    Subscribe,
    /// Subscribe acknowledgment
    SubAck,
    /// Unsubscribe request
    Unsubscribe,
    /// Unsubscribe acknowledgment
    UnsubAck,
    /// PING request
    PingReq,
    /// PING response
    PingResp,
    /// Disconnect notification
    Disconnect,
    /// Authentication exchange
    Auth,
}

impl TryFrom<u8> for PacketType {
    type Error = Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(PacketType::Connect),
            2 => Ok(PacketType::ConnAck),
            3 => Ok(PacketType::Publish),
            4 => Ok(PacketType::PubAck),
            5 => Ok(PacketType::PubRec),
            6 => Ok(PacketType::PubRel),
            7 => Ok(PacketType::PubComp),
            8 => Ok(PacketType::Subscribe),
            9 => Ok(PacketType::SubAck),
            10 => Ok(PacketType::Unsubscribe),
            11 => Ok(PacketType::UnsubAck),
            12 => Ok(PacketType::PingReq),
            13 => Ok(PacketType::PingResp),
            14 => Ok(PacketType::Disconnect),
            15 => Ok(PacketType::Auth),
            x => Err(Error::InvalidPacketType(x)),
        }
    }
}

/// Get the packet size from the remaining length
fn size_from_len(len: crate::VarInt) -> usize {
    // control field + remaining length + variable header & payload
    1 + len.length() + len.value()
}

#[cfg(test)]
mod tests {
    // These are used in tests by packets
    pub const USER_PROP_KEY: &str = "property";
    pub const USER_PROP_VAL: &str = "a value thats really long............................................................................................................";
}
