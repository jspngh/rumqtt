use bytes::Bytes;

use crate::{Properties, QoS};

pub(crate) mod v4;
pub(crate) mod v5;

/// Publish message
///
/// Sent from a client to a server or from a server to a client to transport an application message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Publish {
    pub dup: bool,
    pub qos: QoS,
    pub retain: bool,
    pub topic: String,
    pub pkid: u16,
    pub properties: Properties,
    pub payload: Bytes,
}

impl Publish {
    pub fn new<T: Into<String>, P: Into<Bytes>>(topic: T, qos: QoS, payload: P) -> Self {
        Publish {
            dup: false,
            qos,
            retain: false,
            pkid: 0,
            topic: topic.into(),
            payload: payload.into(),
            properties: Properties::new(),
        }
    }
}
