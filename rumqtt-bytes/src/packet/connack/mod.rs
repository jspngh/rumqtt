use crate::{property::Properties, reason, Error};

pub(crate) mod v4;
pub(crate) mod v5;

/// Connect acknowledgment
///
/// Packet sent by the server in response to a CONNECT packet received from a client.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnAck {
    pub session_present: bool,
    pub code: ConnectReasonCode,
    pub properties: Properties,
}

impl ConnAck {
    /// Create a new ConnAck packet
    pub fn new(session_present: bool) -> Self {
        Self {
            session_present,
            code: ConnectReasonCode::Success,
            properties: Properties::new(),
        }
    }

    /// Create a new ConnAck packet from a [ConnectReturnCode]
    pub fn from_return_code(code: ConnectReturnCode, session_present: bool) -> Self {
        Self {
            session_present,
            code: code.into(),
            properties: Properties::new(),
        }
    }
}

/// MQTT 5.0 reason codes
///
/// A subset of these codes are used in MQTT 3.1.1 as well.
/// This means a [ConnectReturnCode] can always be converted to a [ConnectReasonCode],
/// but the conversion in the other direction is fallible.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ConnectReasonCode {
    Success = reason::SUCCESS,
    UnspecifiedError = reason::UNSPECIFIED_ERROR,
    MalformedPacket = reason::MALFORMED_PACKET,
    ProtocolError = reason::PROTOCOL_ERROR,
    ImplementationSpecificError = reason::IMPLEMENTATION_SPECIFIC_ERROR,
    UnsupportedProtocolVersion = reason::UNSUPPORTED_PROTOCOL_VERSION,
    ClientIdentifierNotValid = reason::CLIENT_IDENTIFIER_NOT_VALID,
    BadUserNamePassword = reason::BAD_USER_NAME_OR_PASSWORD,
    NotAuthorized = reason::NOT_AUTHORIZED,
    ServerUnavailable = reason::SERVER_UNAVAILABLE,
    ServerBusy = reason::SERVER_BUSY,
    Banned = reason::BANNED,
    BadAuthenticationMethod = reason::BAD_AUTHENTICATION_METHOD,
    TopicNameInvalid = reason::TOPIC_NAME_INVALID,
    PacketTooLarge = reason::PACKET_TOO_LARGE,
    QuotaExceeded = reason::QUOTA_EXCEEDED,
    PayloadFormatInvalid = reason::PAYLOAD_FORMAT_INVALID,
    RetainNotSupported = reason::RETAIN_NOT_SUPPORTED,
    QoSNotSupported = reason::QOS_NOT_SUPPORTED,
    UseAnotherServer = reason::USE_ANOTHER_SERVER,
    ServerMoved = reason::SERVER_MOVED,
    ConnectionRateExceeded = reason::CONNECTION_RATE_EXCEEDED,
}

/// MQTT 3.1.1 return codes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ConnectReturnCode {
    Success = 0,
    RefusedProtocolVersion,
    BadClientId,
    ServiceUnavailable,
    BadUserNamePassword,
    NotAuthorized,
}

impl TryFrom<u8> for ConnectReasonCode {
    type Error = Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        let code = match value {
            reason::SUCCESS => Self::Success,
            reason::UNSPECIFIED_ERROR => Self::UnspecifiedError,
            reason::MALFORMED_PACKET => Self::MalformedPacket,
            reason::PROTOCOL_ERROR => Self::ProtocolError,
            reason::IMPLEMENTATION_SPECIFIC_ERROR => Self::ImplementationSpecificError,
            reason::UNSUPPORTED_PROTOCOL_VERSION => Self::UnsupportedProtocolVersion,
            reason::CLIENT_IDENTIFIER_NOT_VALID => Self::ClientIdentifierNotValid,
            reason::BAD_USER_NAME_OR_PASSWORD => Self::BadUserNamePassword,
            reason::NOT_AUTHORIZED => Self::NotAuthorized,
            reason::SERVER_UNAVAILABLE => Self::ServerUnavailable,
            reason::SERVER_BUSY => Self::ServerBusy,
            reason::BANNED => Self::Banned,
            reason::BAD_AUTHENTICATION_METHOD => Self::BadAuthenticationMethod,
            reason::TOPIC_NAME_INVALID => Self::TopicNameInvalid,
            reason::PACKET_TOO_LARGE => Self::PacketTooLarge,
            reason::QUOTA_EXCEEDED => Self::QuotaExceeded,
            reason::PAYLOAD_FORMAT_INVALID => Self::PayloadFormatInvalid,
            reason::RETAIN_NOT_SUPPORTED => Self::RetainNotSupported,
            reason::QOS_NOT_SUPPORTED => Self::QoSNotSupported,
            reason::USE_ANOTHER_SERVER => Self::UseAnotherServer,
            reason::SERVER_MOVED => Self::ServerMoved,
            reason::CONNECTION_RATE_EXCEEDED => Self::ConnectionRateExceeded,
            num => return Err(Error::InvalidConnectReturnCode(num)),
        };

        Ok(code)
    }
}

impl TryFrom<u8> for ConnectReturnCode {
    type Error = Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        let code = match value {
            0 => Self::Success,
            1 => Self::RefusedProtocolVersion,
            2 => Self::BadClientId,
            3 => Self::ServiceUnavailable,
            4 => Self::BadUserNamePassword,
            5 => Self::NotAuthorized,

            num => return Err(Error::InvalidConnectReturnCode(num)),
        };

        Ok(code)
    }
}

impl From<ConnectReturnCode> for ConnectReasonCode {
    fn from(value: ConnectReturnCode) -> Self {
        match value {
            ConnectReturnCode::Success => Self::Success,
            ConnectReturnCode::RefusedProtocolVersion => Self::UnsupportedProtocolVersion,
            ConnectReturnCode::BadClientId => Self::ClientIdentifierNotValid,
            ConnectReturnCode::ServiceUnavailable => Self::ServerUnavailable,
            ConnectReturnCode::BadUserNamePassword => Self::BadUserNamePassword,
            ConnectReturnCode::NotAuthorized => Self::NotAuthorized,
        }
    }
}

impl TryFrom<ConnectReasonCode> for ConnectReturnCode {
    type Error = Error;

    fn try_from(value: ConnectReasonCode) -> Result<Self, Self::Error> {
        let code = match value {
            ConnectReasonCode::Success => Self::Success,
            ConnectReasonCode::UnsupportedProtocolVersion => Self::RefusedProtocolVersion,
            ConnectReasonCode::ClientIdentifierNotValid => Self::BadClientId,
            ConnectReasonCode::ServerUnavailable => Self::ServiceUnavailable,
            ConnectReasonCode::BadUserNamePassword => Self::BadUserNamePassword,
            ConnectReasonCode::NotAuthorized => Self::NotAuthorized,
            _ => {
                // MQTT 3.1.1 does not support all MQTT 5.0 reason codes
                return Err(Error::InvalidConnectReturnCode(value as u8));
            }
        };
        Ok(code)
    }
}
