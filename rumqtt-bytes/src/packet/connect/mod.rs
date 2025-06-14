use bytes::{Buf, BufMut, Bytes, BytesMut};

use crate::parse::*;
use crate::property::PropertyType;
use crate::{Error, QoS};

pub(crate) mod v4;
pub(crate) mod v5;

/// Connection request
///
/// The first packet that must be sent to a server after a client establishes a network connection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Connect {
    /// MQTT keep alive time
    pub keep_alive: u16,
    /// Clean session. Asks the broker to clear previous state
    pub clean_start: bool,
    /// Properties of the connect packet
    pub properties: Option<ConnectProperties>,
    /// Client Identifier - must be present in the payload
    pub client_id: String,
    /// Will message that broker needs to publish when the client disconnects
    pub last_will: Option<LastWill>,
    /// Login credentials
    pub login: Option<Login>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ConnectProperties {
    /// Expiry interval property after loosing connection
    pub session_expiry_interval: Option<u32>,
    /// Maximum simultaneous packets
    pub receive_maximum: Option<u16>,
    /// Maximum packet size
    pub max_packet_size: Option<u32>,
    /// Maximum mapping integer for a topic
    pub topic_alias_max: Option<u16>,
    pub request_response_info: Option<u8>,
    pub request_problem_info: Option<u8>,
    /// List of user properties
    pub user_properties: Vec<(String, String)>,
    /// Method of authentication
    pub authentication_method: Option<String>,
    /// Authentication data
    pub authentication_data: Option<Bytes>,
}

/// LastWill that broker forwards on behalf of the client
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LastWill {
    pub topic: String,
    pub payload: Bytes,
    pub qos: QoS,
    pub retain: bool,
    pub properties: Option<LastWillProperties>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LastWillProperties {
    pub delay_interval: Option<u32>,
    pub payload_format_indicator: Option<u8>,
    pub message_expiry_interval: Option<u32>,
    pub content_type: Option<String>,
    pub response_topic: Option<String>,
    pub correlation_data: Option<Bytes>,
    pub user_properties: Vec<(String, String)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Login {
    pub username: String,
    pub password: String, // FIXME: this should be binary data, not a string
}

impl Connect {
    pub fn new(keep_alive: u16, clean_start: bool, client_id: impl Into<String>) -> Self {
        Self {
            keep_alive,
            clean_start,
            properties: None,
            client_id: client_id.into(),
            last_will: None,
            login: None,
        }
    }

    pub fn set_login<U: Into<String>, P: Into<String>>(&mut self, u: U, p: P) -> &mut Connect {
        let login = Login {
            username: u.into(),
            password: p.into(),
        };

        self.login = Some(login);
        self
    }
}

impl ConnectProperties {
    fn len(&self) -> Result<VarInt, Error> {
        let mut len = 0;

        if self.session_expiry_interval.is_some() {
            len += 1 + 4;
        }

        if self.receive_maximum.is_some() {
            len += 1 + 2;
        }

        if self.max_packet_size.is_some() {
            len += 1 + 4;
        }

        if self.topic_alias_max.is_some() {
            len += 1 + 2;
        }

        if self.request_response_info.is_some() {
            len += 1 + 1;
        }

        if self.request_problem_info.is_some() {
            len += 1 + 1;
        }

        for (key, value) in self.user_properties.iter() {
            len += 1 + 2 + key.len() + 2 + value.len();
        }

        if let Some(authentication_method) = &self.authentication_method {
            len += 1 + 2 + authentication_method.len();
        }

        if let Some(authentication_data) = &self.authentication_data {
            len += 1 + 2 + authentication_data.len();
        }

        VarInt::new(len)
    }

    pub fn read(bytes: &mut Bytes) -> Result<Option<ConnectProperties>, Error> {
        let mut session_expiry_interval = None;
        let mut receive_maximum = None;
        let mut max_packet_size = None;
        let mut topic_alias_max = None;
        let mut request_response_info = None;
        let mut request_problem_info = None;
        let mut user_properties = Vec::new();
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
            match PropertyType::try_from(prop)? {
                PropertyType::SessionExpiryInterval => {
                    session_expiry_interval = Some(read_u32(bytes)?);
                    cursor += 4;
                }
                PropertyType::ReceiveMaximum => {
                    receive_maximum = Some(read_u16(bytes)?);
                    cursor += 2;
                }
                PropertyType::MaximumPacketSize => {
                    max_packet_size = Some(read_u32(bytes)?);
                    cursor += 4;
                }
                PropertyType::TopicAliasMaximum => {
                    topic_alias_max = Some(read_u16(bytes)?);
                    cursor += 2;
                }
                PropertyType::RequestResponseInformation => {
                    request_response_info = Some(read_u8(bytes)?);
                    cursor += 1;
                }
                PropertyType::RequestProblemInformation => {
                    request_problem_info = Some(read_u8(bytes)?);
                    cursor += 1;
                }
                PropertyType::UserProperty => {
                    let key = read_mqtt_string(bytes)?;
                    let value = read_mqtt_string(bytes)?;
                    cursor += 2 + key.len() + 2 + value.len();
                    user_properties.push((key, value));
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

        Ok(Some(ConnectProperties {
            session_expiry_interval,
            receive_maximum,
            max_packet_size,
            topic_alias_max,
            request_response_info,
            request_problem_info,
            user_properties,
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

        if let Some(receive_maximum) = self.receive_maximum {
            buffer.put_u8(PropertyType::ReceiveMaximum as u8);
            buffer.put_u16(receive_maximum);
        }

        if let Some(max_packet_size) = self.max_packet_size {
            buffer.put_u8(PropertyType::MaximumPacketSize as u8);
            buffer.put_u32(max_packet_size);
        }

        if let Some(topic_alias_max) = self.topic_alias_max {
            buffer.put_u8(PropertyType::TopicAliasMaximum as u8);
            buffer.put_u16(topic_alias_max);
        }

        if let Some(request_response_info) = self.request_response_info {
            buffer.put_u8(PropertyType::RequestResponseInformation as u8);
            buffer.put_u8(request_response_info);
        }

        if let Some(request_problem_info) = self.request_problem_info {
            buffer.put_u8(PropertyType::RequestProblemInformation as u8);
            buffer.put_u8(request_problem_info);
        }

        for (key, value) in self.user_properties.iter() {
            buffer.put_u8(PropertyType::UserProperty as u8);
            write_mqtt_string(buffer, key);
            write_mqtt_string(buffer, value);
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

impl LastWillProperties {
    fn len(&self) -> Result<VarInt, Error> {
        let mut len = 0;

        if self.delay_interval.is_some() {
            len += 1 + 4;
        }

        if self.payload_format_indicator.is_some() {
            len += 1 + 1;
        }

        if self.message_expiry_interval.is_some() {
            len += 1 + 4;
        }

        if let Some(typ) = &self.content_type {
            len += 1 + 2 + typ.len()
        }

        if let Some(topic) = &self.response_topic {
            len += 1 + 2 + topic.len()
        }

        if let Some(data) = &self.correlation_data {
            len += 1 + 2 + data.len()
        }

        for (key, value) in self.user_properties.iter() {
            len += 1 + 2 + key.len() + 2 + value.len();
        }

        VarInt::new(len)
    }

    pub fn read(bytes: &mut Bytes) -> Result<Option<LastWillProperties>, Error> {
        let mut delay_interval = None;
        let mut payload_format_indicator = None;
        let mut message_expiry_interval = None;
        let mut content_type = None;
        let mut response_topic = None;
        let mut correlation_data = None;
        let mut user_properties = Vec::new();

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
                PropertyType::WillDelayInterval => {
                    delay_interval = Some(read_u32(bytes)?);
                    cursor += 4;
                }
                PropertyType::PayloadFormatIndicator => {
                    payload_format_indicator = Some(read_u8(bytes)?);
                    cursor += 1;
                }
                PropertyType::MessageExpiryInterval => {
                    message_expiry_interval = Some(read_u32(bytes)?);
                    cursor += 4;
                }
                PropertyType::ContentType => {
                    let typ = read_mqtt_string(bytes)?;
                    cursor += 2 + typ.len();
                    content_type = Some(typ);
                }
                PropertyType::ResponseTopic => {
                    let topic = read_mqtt_string(bytes)?;
                    cursor += 2 + topic.len();
                    response_topic = Some(topic);
                }
                PropertyType::CorrelationData => {
                    let data = read_mqtt_bytes(bytes)?;
                    cursor += 2 + data.len();
                    correlation_data = Some(data);
                }
                PropertyType::UserProperty => {
                    let key = read_mqtt_string(bytes)?;
                    let value = read_mqtt_string(bytes)?;
                    cursor += 2 + key.len() + 2 + value.len();
                    user_properties.push((key, value));
                }
                _ => return Err(Error::InvalidPropertyType(prop)),
            }
        }

        Ok(Some(LastWillProperties {
            delay_interval,
            payload_format_indicator,
            message_expiry_interval,
            content_type,
            response_topic,
            correlation_data,
            user_properties,
        }))
    }

    pub fn write(&self, buffer: &mut BytesMut) -> Result<(), Error> {
        let len = self.len()?;
        len.write(buffer);

        if let Some(delay_interval) = self.delay_interval {
            buffer.put_u8(PropertyType::WillDelayInterval as u8);
            buffer.put_u32(delay_interval);
        }

        if let Some(payload_format_indicator) = self.payload_format_indicator {
            buffer.put_u8(PropertyType::PayloadFormatIndicator as u8);
            buffer.put_u8(payload_format_indicator);
        }

        if let Some(message_expiry_interval) = self.message_expiry_interval {
            buffer.put_u8(PropertyType::MessageExpiryInterval as u8);
            buffer.put_u32(message_expiry_interval);
        }

        if let Some(typ) = &self.content_type {
            buffer.put_u8(PropertyType::ContentType as u8);
            write_mqtt_string(buffer, typ);
        }

        if let Some(topic) = &self.response_topic {
            buffer.put_u8(PropertyType::ResponseTopic as u8);
            write_mqtt_string(buffer, topic);
        }

        if let Some(data) = &self.correlation_data {
            buffer.put_u8(PropertyType::CorrelationData as u8);
            write_mqtt_bytes(buffer, data);
        }

        for (key, value) in self.user_properties.iter() {
            buffer.put_u8(PropertyType::UserProperty as u8);
            write_mqtt_string(buffer, key);
            write_mqtt_string(buffer, value);
        }

        Ok(())
    }
}

impl Login {
    pub fn new<U: Into<String>, P: Into<String>>(u: U, p: P) -> Login {
        Login {
            username: u.into(),
            password: p.into(),
        }
    }

    pub fn read(connect_flags: u8, bytes: &mut Bytes) -> Result<Option<Login>, Error> {
        let username = match connect_flags & 0b1000_0000 {
            0 => String::new(),
            _ => read_mqtt_string(bytes)?,
        };

        let password = match connect_flags & 0b0100_0000 {
            0 => String::new(),
            _ => read_mqtt_string(bytes)?,
        };

        if username.is_empty() && password.is_empty() {
            Ok(None)
        } else {
            Ok(Some(Login { username, password }))
        }
    }

    pub fn write(&self, buffer: &mut BytesMut) -> u8 {
        let mut connect_flags = 0;
        if !self.username.is_empty() {
            connect_flags |= 0x80;
            write_mqtt_string(buffer, &self.username);
        }

        if !self.password.is_empty() {
            connect_flags |= 0x40;
            write_mqtt_string(buffer, &self.password);
        }

        connect_flags
    }

    fn len(&self) -> usize {
        let mut len = 0;

        if !self.username.is_empty() {
            len += 2 + self.username.len();
        }

        if !self.password.is_empty() {
            len += 2 + self.password.len();
        }

        len
    }
}
