use bytes::{Bytes, BytesMut};

use crate::parse::*;
use crate::{Error, Properties, QoS};

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
    pub properties: Properties,
    /// Client Identifier - must be present in the payload
    pub client_id: String,
    /// Will message that broker needs to publish when the client disconnects
    pub last_will: Option<Box<LastWill>>,
    /// Login credentials
    pub login: Option<Box<Login>>,
}

impl Connect {
    pub fn new(keep_alive: u16, clean_start: bool, client_id: impl Into<String>) -> Self {
        Self {
            keep_alive,
            clean_start,
            properties: Properties::new(),
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

        self.login = Some(Box::new(login));
        self
    }
}

/// LastWill that broker forwards on behalf of the client
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LastWill {
    pub qos: QoS,
    pub retain: bool,
    pub properties: Properties,
    pub topic: String,
    pub payload: Bytes,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Login {
    pub username: String,
    pub password: String, // FIXME: this should be binary data, not a string
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
