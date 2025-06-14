use bytes::{Buf, BufMut, Bytes, BytesMut};

use crate::{parse::*, property::PropertyType, reason, Error};

pub(crate) mod v4;
pub(crate) mod v5;

/// Connect acknowledgment
///
/// Packet sent by the server in response to a CONNECT packet received from a client.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnAck {
    pub session_present: bool,
    pub code: ConnectReasonCode,
    pub properties: Option<ConnAckProperties>,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnAckProperties {
    pub session_expiry_interval: Option<u32>,
    pub receive_max: Option<u16>,
    pub max_qos: Option<u8>,
    pub retain_available: Option<u8>,
    pub max_packet_size: Option<u32>,
    pub assigned_client_identifier: Option<String>,
    pub topic_alias_max: Option<u16>,
    pub reason_string: Option<String>,
    pub user_properties: Vec<(String, String)>,
    pub wildcard_subscription_available: Option<u8>,
    pub subscription_identifiers_available: Option<u8>,
    pub shared_subscription_available: Option<u8>,
    pub server_keep_alive: Option<u16>,
    pub response_information: Option<String>,
    pub server_reference: Option<String>,
    pub authentication_method: Option<String>,
    pub authentication_data: Option<Bytes>,
}

impl ConnAck {
    /// Create a new ConnAck packet
    pub fn new(session_present: bool) -> Self {
        Self {
            session_present,
            code: ConnectReasonCode::Success,
            properties: None,
        }
    }

    /// Create a new ConnAck packet from a [ConnectReturnCode]
    pub fn from_return_code(code: ConnectReturnCode, session_present: bool) -> Self {
        Self {
            session_present,
            code: code.into(),
            properties: None,
        }
    }
}

impl ConnAckProperties {
    fn len(&self) -> Result<VarInt, Error> {
        let mut len = 0;

        if self.session_expiry_interval.is_some() {
            len += 1 + 4;
        }

        if self.receive_max.is_some() {
            len += 1 + 2;
        }

        if self.max_qos.is_some() {
            len += 1 + 1;
        }

        if self.retain_available.is_some() {
            len += 1 + 1;
        }

        if self.max_packet_size.is_some() {
            len += 1 + 4;
        }

        if let Some(id) = &self.assigned_client_identifier {
            len += 1 + 2 + id.len();
        }

        if self.topic_alias_max.is_some() {
            len += 1 + 2;
        }

        if let Some(reason) = &self.reason_string {
            len += 1 + 2 + reason.len();
        }

        for (key, value) in self.user_properties.iter() {
            len += 1 + 2 + key.len() + 2 + value.len();
        }

        if self.wildcard_subscription_available.is_some() {
            len += 1 + 1;
        }

        if self.subscription_identifiers_available.is_some() {
            len += 1 + 1;
        }

        if self.shared_subscription_available.is_some() {
            len += 1 + 1;
        }

        if self.server_keep_alive.is_some() {
            len += 1 + 2;
        }

        if let Some(info) = &self.response_information {
            len += 1 + 2 + info.len();
        }

        if let Some(reference) = &self.server_reference {
            len += 1 + 2 + reference.len();
        }

        if let Some(authentication_method) = &self.authentication_method {
            len += 1 + 2 + authentication_method.len();
        }

        if let Some(authentication_data) = &self.authentication_data {
            len += 1 + 2 + authentication_data.len();
        }

        VarInt::new(len)
    }

    pub fn read(bytes: &mut Bytes) -> Result<Option<ConnAckProperties>, Error> {
        let mut session_expiry_interval = None;
        let mut receive_max = None;
        let mut max_qos = None;
        let mut retain_available = None;
        let mut max_packet_size = None;
        let mut assigned_client_identifier = None;
        let mut topic_alias_max = None;
        let mut reason_string = None;
        let mut user_properties = Vec::new();
        let mut wildcard_subscription_available = None;
        let mut subscription_identifiers_available = None;
        let mut shared_subscription_available = None;
        let mut server_keep_alive = None;
        let mut response_information = None;
        let mut server_reference = None;
        let mut authentication_method = None;
        let mut authentication_data = None;

        let properties_len = VarInt::read(bytes.iter())?;
        bytes.advance(properties_len.length());
        if properties_len == 0 {
            return Ok(None);
        }

        let mut cursor = 0;
        // read until cursor reaches property length. properties_len = 0 will skip this loop
        while properties_len > cursor {
            let prop = read_u8(bytes)?;
            cursor += 1;

            match prop.try_into()? {
                PropertyType::SessionExpiryInterval => {
                    session_expiry_interval = Some(read_u32(bytes)?);
                    cursor += 4;
                }
                PropertyType::ReceiveMaximum => {
                    receive_max = Some(read_u16(bytes)?);
                    cursor += 2;
                }
                PropertyType::MaximumQos => {
                    max_qos = Some(read_u8(bytes)?);
                    cursor += 1;
                }
                PropertyType::RetainAvailable => {
                    retain_available = Some(read_u8(bytes)?);
                    cursor += 1;
                }
                PropertyType::AssignedClientIdentifier => {
                    let id = read_mqtt_string(bytes)?;
                    cursor += 2 + id.len();
                    assigned_client_identifier = Some(id);
                }
                PropertyType::MaximumPacketSize => {
                    max_packet_size = Some(read_u32(bytes)?);
                    cursor += 4;
                }
                PropertyType::TopicAliasMaximum => {
                    topic_alias_max = Some(read_u16(bytes)?);
                    cursor += 2;
                }
                PropertyType::ReasonString => {
                    let reason = read_mqtt_string(bytes)?;
                    cursor += 2 + reason.len();
                    reason_string = Some(reason);
                }
                PropertyType::UserProperty => {
                    let key = read_mqtt_string(bytes)?;
                    let value = read_mqtt_string(bytes)?;
                    cursor += 2 + key.len() + 2 + value.len();
                    user_properties.push((key, value));
                }
                PropertyType::WildcardSubscriptionAvailable => {
                    wildcard_subscription_available = Some(read_u8(bytes)?);
                    cursor += 1;
                }
                PropertyType::SubscriptionIdentifierAvailable => {
                    subscription_identifiers_available = Some(read_u8(bytes)?);
                    cursor += 1;
                }
                PropertyType::SharedSubscriptionAvailable => {
                    shared_subscription_available = Some(read_u8(bytes)?);
                    cursor += 1;
                }
                PropertyType::ServerKeepAlive => {
                    server_keep_alive = Some(read_u16(bytes)?);
                    cursor += 2;
                }
                PropertyType::ResponseInformation => {
                    let info = read_mqtt_string(bytes)?;
                    cursor += 2 + info.len();
                    response_information = Some(info);
                }
                PropertyType::ServerReference => {
                    let reference = read_mqtt_string(bytes)?;
                    cursor += 2 + reference.len();
                    server_reference = Some(reference);
                }
                PropertyType::AuthenticationMethod => {
                    let method = read_mqtt_string(bytes)?;
                    cursor += 2 + method.len();
                    authentication_method = Some(method);
                }
                PropertyType::AuthenticationData => {
                    let data = read_mqtt_bytes(bytes)?;
                    cursor += 2 + data.len();
                    authentication_data = Some(data);
                }
                _ => return Err(Error::InvalidPropertyType(prop)),
            }
        }

        Ok(Some(ConnAckProperties {
            session_expiry_interval,
            receive_max,
            max_qos,
            retain_available,
            max_packet_size,
            assigned_client_identifier,
            topic_alias_max,
            reason_string,
            user_properties,
            wildcard_subscription_available,
            subscription_identifiers_available,
            shared_subscription_available,
            server_keep_alive,
            response_information,
            server_reference,
            authentication_method,
            authentication_data,
        }))
    }

    pub fn write(&self, buffer: &mut BytesMut) -> Result<(), Error> {
        let len = self.len()?;
        len.write(buffer);

        if let Some(session_expiry_interval) = self.session_expiry_interval {
            buffer.put_u8(PropertyType::SessionExpiryInterval as u8);
            buffer.put_u32(session_expiry_interval);
        }

        if let Some(receive_maximum) = self.receive_max {
            buffer.put_u8(PropertyType::ReceiveMaximum as u8);
            buffer.put_u16(receive_maximum);
        }

        if let Some(qos) = self.max_qos {
            buffer.put_u8(PropertyType::MaximumQos as u8);
            buffer.put_u8(qos);
        }

        if let Some(retain_available) = self.retain_available {
            buffer.put_u8(PropertyType::RetainAvailable as u8);
            buffer.put_u8(retain_available);
        }

        if let Some(max_packet_size) = self.max_packet_size {
            buffer.put_u8(PropertyType::MaximumPacketSize as u8);
            buffer.put_u32(max_packet_size);
        }

        if let Some(id) = &self.assigned_client_identifier {
            buffer.put_u8(PropertyType::AssignedClientIdentifier as u8);
            write_mqtt_string(buffer, id);
        }

        if let Some(topic_alias_max) = self.topic_alias_max {
            buffer.put_u8(PropertyType::TopicAliasMaximum as u8);
            buffer.put_u16(topic_alias_max);
        }

        if let Some(reason) = &self.reason_string {
            buffer.put_u8(PropertyType::ReasonString as u8);
            write_mqtt_string(buffer, reason);
        }

        for (key, value) in self.user_properties.iter() {
            buffer.put_u8(PropertyType::UserProperty as u8);
            write_mqtt_string(buffer, key);
            write_mqtt_string(buffer, value);
        }

        if let Some(w) = self.wildcard_subscription_available {
            buffer.put_u8(PropertyType::WildcardSubscriptionAvailable as u8);
            buffer.put_u8(w);
        }

        if let Some(s) = self.subscription_identifiers_available {
            buffer.put_u8(PropertyType::SubscriptionIdentifierAvailable as u8);
            buffer.put_u8(s);
        }

        if let Some(s) = self.shared_subscription_available {
            buffer.put_u8(PropertyType::SharedSubscriptionAvailable as u8);
            buffer.put_u8(s);
        }

        if let Some(keep_alive) = self.server_keep_alive {
            buffer.put_u8(PropertyType::ServerKeepAlive as u8);
            buffer.put_u16(keep_alive);
        }

        if let Some(info) = &self.response_information {
            buffer.put_u8(PropertyType::ResponseInformation as u8);
            write_mqtt_string(buffer, info);
        }

        if let Some(reference) = &self.server_reference {
            buffer.put_u8(PropertyType::ServerReference as u8);
            write_mqtt_string(buffer, reference);
        }

        if let Some(authentication_method) = &self.authentication_method {
            buffer.put_u8(PropertyType::AuthenticationMethod as u8);
            write_mqtt_string(buffer, authentication_method);
        }

        if let Some(authentication_data) = &self.authentication_data {
            buffer.put_u8(PropertyType::AuthenticationData as u8);
            write_mqtt_bytes(buffer, authentication_data);
        }

        Ok(())
    }
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
