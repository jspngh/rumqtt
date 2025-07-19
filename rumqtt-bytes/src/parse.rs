//! Parsing utility functions

use std::slice::Iter;

use bytes::{Buf, BufMut, Bytes, BytesMut};

use crate::Error;

/// Variable Byte Integer
///
/// An unsigned integer that is encoded in one to four bytes.
///
/// In MQTT 3.1.1, it is used only for the `Remaining Length` in the fixed header.
/// In MQTT 5.0, it is formalized and used in multiple places.
///
/// See [specification](https://docs.oasis-open.org/mqtt/mqtt/v5.0/os/mqtt-v5.0-os.html#_Toc3901011).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct VarInt {
    value: u32,
    length: u8,
}

impl PartialOrd<usize> for VarInt {
    fn partial_cmp(&self, other: &usize) -> Option<std::cmp::Ordering> {
        Some(self.value().cmp(other))
    }
}

impl PartialEq<usize> for VarInt {
    fn eq(&self, other: &usize) -> bool {
        self.value().eq(other)
    }
}

impl From<VarInt> for u32 {
    fn from(val: VarInt) -> Self {
        val.value
    }
}

impl VarInt {
    /// Creates a new variable byte integer
    ///
    /// # Errors
    /// This will return an error if the value is too large to be encoded
    pub fn new(value: usize) -> Result<Self, Error> {
        let length = match value {
            0..=127 => 1,
            128..=16_383 => 2,
            16_384..=2_097_151 => 3,
            2_097_152..=268_435_455 => 4,
            _ => return Err(Error::PayloadTooLong),
        };
        Ok(Self {
            value: value as u32,
            length,
        })
    }

    /// Creates a variable byte integer with a compile-time-known value
    ///
    /// This function provides a slightly easier to use interface for when
    /// you know at compile time that the value is within the valid range.
    ///
    /// # Panics
    /// If this function executes at runtime, it will panic with a value greater than `268_435_455`.
    pub const fn constant(value: usize) -> Self {
        let length = match value {
            0..=127 => 1,
            128..=16_383 => 2,
            16_384..=2_097_151 => 3,
            2_097_152..=268_435_455 => 4,
            _ => panic!("value should be < 268_435_456"),
        };
        Self {
            value: value as u32,
            length,
        }
    }

    /// The numeric value of the variable byte integer
    pub const fn value(&self) -> usize {
        self.value as usize
    }

    /// The number of bytes required to encode this variable byte integer
    pub const fn length(&self) -> usize {
        self.length as usize
    }

    /// Read a variable byte integer from the stream
    ///
    /// This returns the variable byte integer and the number of bytes that have been read
    pub fn read(stream: Iter<u8>) -> Result<Self, Error> {
        let mut value: u32 = 0;
        let mut length = 0;
        let mut shift = 0;
        let mut done = false;

        // Use continuation bit at position 7 to continue reading next byte to frame 'length'.
        // Stream 0b1xxx_xxxx 0b1yyy_yyyy 0b1zzz_zzzz 0b0www_wwww will
        // be framed as number 0bwww_wwww_zzz_zzzz_yyy_yyyy_xxx_xxxx
        for &byte in stream {
            value += ((byte & 0b0111_1111) as u32) << shift;
            length += 1;
            shift += 7;

            // stop when continuation bit is 0
            if (byte & 0b1000_0000) == 0 {
                done = true;
                break;
            }

            // Only a max of 4 bytes allowed for remaining length
            // We should have already exited the loop
            if length >= 4 {
                return Err(Error::MalformedRemainingLength);
            }
        }

        // Not enough bytes in stream to frame remaining length
        // Wait for at least one more byte
        if !done {
            return Err(Error::InsufficientBytes(1));
        }

        Ok(Self { value, length })
    }

    /// Write a variable byte integer to the stream
    pub fn write(&self, stream: &mut BytesMut) {
        let mut x = self.value;
        let mut done = false;

        while !done {
            let mut byte = (x % 128) as u8;
            x >>= 7;
            if x > 0 {
                byte |= 128;
            } else {
                done = true;
            }

            stream.put_u8(byte);
        }
    }
}

/// Read [Binary Data][1] from a byte stream.
///
/// [1]: https://docs.oasis-open.org/mqtt/mqtt/v5.0/os/mqtt-v5.0-os.html#_Toc3901012
pub fn read_mqtt_bytes(stream: &mut Bytes) -> Result<Bytes, Error> {
    let len = read_u16(stream)? as usize;

    // Prevent attacks with wrong remaining length. This method is used in
    // `packet.assembly()` with (enough) bytes to frame packet. Ensures that
    // reading variable len string or bytes doesn't cross promised boundary
    // with `read_fixed_header()`
    if len > stream.len() {
        return Err(Error::BoundaryCrossed(len));
    }

    Ok(stream.split_to(len))
}

/// Read a [UTF-8 Encoded String][1] from a byte stream.
///
/// [1]: https://docs.oasis-open.org/mqtt/mqtt/v5.0/os/mqtt-v5.0-os.html#_Toc3901010
pub fn read_mqtt_string(stream: &mut Bytes) -> Result<String, Error> {
    let s = read_mqtt_bytes(stream)?;
    String::from_utf8(s.to_vec()).map_err(|e| Error::Utf8Encoding(e.utf8_error()))
}

/// Write [Binary Data][1] to a byte stream.
///
/// [1]: https://docs.oasis-open.org/mqtt/mqtt/v5.0/os/mqtt-v5.0-os.html#_Toc3901012
pub fn write_mqtt_bytes(stream: &mut BytesMut, bytes: &[u8]) {
    stream.put_u16(bytes.len() as u16);
    stream.extend_from_slice(bytes);
}

/// Write a [UTF-8 Encoded String][1] to a byte stream.
///
/// [1]: https://docs.oasis-open.org/mqtt/mqtt/v5.0/os/mqtt-v5.0-os.html#_Toc3901010
pub fn write_mqtt_string(stream: &mut BytesMut, string: &str) {
    write_mqtt_bytes(stream, string.as_bytes());
}

/// A checked version of [`bytes::Buf::get_u8`]
pub fn read_u8(stream: &mut Bytes) -> Result<u8, Error> {
    if stream.is_empty() {
        return Err(Error::MalformedPacket);
    }

    Ok(stream.get_u8())
}

/// A checked version of [`bytes::Buf::get_u16`]
pub fn read_u16(stream: &mut Bytes) -> Result<u16, Error> {
    if stream.len() < 2 {
        return Err(Error::MalformedPacket);
    }

    Ok(stream.get_u16())
}

/// A checked version of [`bytes::Buf::get_u32`]
pub fn read_u32(stream: &mut Bytes) -> Result<u32, Error> {
    if stream.len() < 4 {
        return Err(Error::MalformedPacket);
    }

    Ok(stream.get_u32())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestCase {
        bytes: Vec<u8>,
        value: usize,
        length: usize,
    }

    #[rustfmt::skip]
    fn test_cases() -> Vec<TestCase> {
        vec![
            TestCase { bytes: vec![0x00], value: 0, length: 1 },
            TestCase { bytes: vec![0x7F], value: 127, length: 1 },
            TestCase { bytes: vec![0x80, 0x01], value: 128, length: 2 },
            TestCase { bytes: vec![0xFF, 0x7F], value: 16_383, length: 2 },
            TestCase { bytes: vec![0x80, 0x80, 0x01], value: 16_384, length: 3 },
            TestCase { bytes: vec![0xFF, 0xFF, 0x7F], value: 2_097_151, length: 3 },
            TestCase { bytes: vec![0x80, 0x80, 0x80, 0x01], value: 2_097_152, length: 4 },
            TestCase { bytes: vec![0xFF, 0xFF, 0xFF, 0x7F], value: 268_435_455, length: 4 },
        ]
    }

    #[test]
    fn test_varint_read() {
        for case in test_cases() {
            let varint = VarInt::read(case.bytes.iter()).unwrap();
            assert_eq!(varint.value(), case.value);
            assert_eq!(varint.length(), case.length);
        }
    }

    #[test]
    fn test_varint_read_unsufficient() {
        let stream = [0x80, 0x80].iter();
        assert!(matches!(
            VarInt::read(stream),
            Err(Error::InsufficientBytes(1))
        ));
    }

    #[test]
    fn test_varint_read_malformed() {
        let stream = [0x80, 0x80, 0x80, 0x80].iter();
        assert!(matches!(
            VarInt::read(stream),
            Err(Error::MalformedRemainingLength)
        ));
    }

    #[test]
    fn test_varint_write() {
        for case in test_cases() {
            let mut stream = BytesMut::new();
            let varint = VarInt::new(case.value).unwrap();
            assert_eq!(varint.length(), case.length);
            varint.write(&mut stream);
            assert_eq!(stream, case.bytes);
        }
    }
}
