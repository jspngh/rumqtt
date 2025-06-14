//! Functionality for dealing with fixed headers of MQTT packets.

use std::slice::Iter;

use crate::parse::*;
use crate::{Error, PacketType};

/// Fixed header of an MQTT Control Packet
///
/// See [specification](https://docs.oasis-open.org/mqtt/mqtt/v5.0/os/mqtt-v5.0-os.html#_Toc3901021).
///
/// ```text
///           7                          3                          0
///           +--------------------------+--------------------------+
/// byte 1    | MQTT Control Packet Type |   Flags for each type    |
///           +--------------------------+--------------------------+
/// bytes 2.. |            Remaining Length (1 to 4 bytes)          |
///           +-----------------------------------------------------+
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd)]
pub struct FixedHeader {
    /// Contains the packet type and several flags
    pub control_field: u8,
    /// Remaining length of the packet.
    ///
    /// This does not include the fixed header bytes.
    /// It represents the variable header + payload.
    pub remaining_len: VarInt,
}

impl FixedHeader {
    pub fn new(byte1: u8, remaining_len: VarInt) -> FixedHeader {
        FixedHeader {
            control_field: byte1,
            remaining_len,
        }
    }

    /// Parse the [PacketType] from the control field
    pub fn packet_type(&self) -> Result<PacketType, Error> {
        PacketType::try_from(self.control_field >> 4)
    }

    /// Get the flag bits from the control field
    #[inline]
    pub fn flags(&self) -> u8 {
        self.control_field & 0x0F
    }

    /// Returns the size of the fixed header
    #[inline]
    pub fn size(&self) -> usize {
        1 + self.remaining_len.length()
    }

    /// Returns the size of full packet (fixed header + variable header + payload)
    ///
    /// Fixed header is enough to get the size of a frame in the stream
    #[inline]
    pub fn packet_size(&self) -> usize {
        self.size() + self.remaining_len.value()
    }

    /// Checks if the stream has enough bytes to frame a packet and returns fixed header
    /// only if a packet can be framed with existing bytes in the `stream`.
    ///
    /// The passed stream doesn't modify parent stream's cursor.
    /// If this function returned an error, next `check` on the same parent stream
    /// is forced start with cursor at 0 again.
    pub fn check(stream: Iter<u8>, max_packet_size: u32) -> Result<Self, Error> {
        let stream_len = stream.len();
        let fixed_header = Self::parse_fixed_header(stream)?;

        // Don't let rogue connections attack with huge payloads.
        // Disconnect them before reading all that data
        if fixed_header.remaining_len > max_packet_size as usize {
            return Err(Error::PayloadSizeLimitExceeded {
                pkt_size: fixed_header.remaining_len.into(),
                max: max_packet_size,
            });
        }

        // If the current call fails due to insufficient bytes in the stream,
        // after calculating remaining length, we extend the stream
        let frame_length = fixed_header.packet_size();
        if stream_len < frame_length {
            return Err(Error::InsufficientBytes(frame_length - stream_len));
        }

        Ok(fixed_header)
    }

    /// Tries to read a [FixedHeader] from the bytestream
    fn parse_fixed_header(mut stream: Iter<u8>) -> Result<Self, Error> {
        let byte1 = stream.next().ok_or(Error::InsufficientBytes(2))?;
        let remaining_len = VarInt::read(stream)?;

        Ok(Self::new(*byte1, remaining_len))
    }
}
