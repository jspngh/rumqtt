use bytes::{BufMut, Bytes, BytesMut};

use crate::{Error, FixedHeader, VarInt};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PingReq;

pub mod req {
    use super::*;

    pub fn read(_fixed_header: FixedHeader, _bytes: Bytes) -> Result<PingReq, Error> {
        Ok(PingReq {})
    }

    pub fn write(_packet: &PingReq, buffer: &mut BytesMut) -> Result<usize, Error> {
        buffer.put_slice(&[0xC0, 0x00]);
        Ok(2)
    }

    pub fn len(_packet: &PingReq) -> Result<VarInt, Error> {
        VarInt::new(0) // no payload
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PingResp;

pub mod resp {
    use super::*;

    pub fn read(_fixed_header: FixedHeader, _bytes: Bytes) -> Result<PingResp, Error> {
        Ok(PingResp {})
    }

    pub fn write(_packet: &PingResp, buffer: &mut BytesMut) -> Result<usize, Error> {
        buffer.put_slice(&[0xD0, 0x00]);
        Ok(2)
    }

    pub fn len(_packet: &PingResp) -> Result<VarInt, Error> {
        VarInt::new(0) // no payload
    }
}
