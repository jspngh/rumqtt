use crate::property::Properties;
use crate::{reason, Error};

pub(crate) mod v5;

/// Authentication exchange
///
/// Sent from client to server or server to client as part of an extended authentication exchange.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Auth {
    pub code: AuthReasonCode,
    pub properties: Properties,
}

impl Auth {
    pub fn new(code: AuthReasonCode) -> Self {
        Self {
            code,
            properties: Properties::new(),
        }
    }
}

/// Auth packet reason code
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum AuthReasonCode {
    Success = reason::SUCCESS,
    Continue = reason::CONTINUE_AUTHENTICATION,
    ReAuthentivate = reason::RE_AUTHENTICATE,
}

impl TryFrom<u8> for AuthReasonCode {
    type Error = Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        let code = match value {
            reason::SUCCESS => AuthReasonCode::Success,
            reason::CONTINUE_AUTHENTICATION => AuthReasonCode::Continue,
            reason::RE_AUTHENTICATE => AuthReasonCode::ReAuthentivate,
            _ => return Err(Error::MalformedPacket),
        };
        Ok(code)
    }
}
