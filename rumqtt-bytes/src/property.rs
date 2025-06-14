//! MQTT 5.0 Property
//!
//! This is a key-value pair that can be included in the variable header of packets.
//! A property consists of an identifier ([PropertyType]), encoded as a Variable Byte Integer,
//! followed by a value.

use crate::Error;

/// Identifiers of the different properties used in MQTT 5.0
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PropertyType {
    PayloadFormatIndicator = 1,
    MessageExpiryInterval = 2,
    ContentType = 3,
    ResponseTopic = 8,
    CorrelationData = 9,
    SubscriptionIdentifier = 11,
    SessionExpiryInterval = 17,
    AssignedClientIdentifier = 18,
    ServerKeepAlive = 19,
    AuthenticationMethod = 21,
    AuthenticationData = 22,
    RequestProblemInformation = 23,
    WillDelayInterval = 24,
    RequestResponseInformation = 25,
    ResponseInformation = 26,
    ServerReference = 28,
    ReasonString = 31,
    ReceiveMaximum = 33,
    TopicAliasMaximum = 34,
    TopicAlias = 35,
    MaximumQos = 36,
    RetainAvailable = 37,
    UserProperty = 38,
    MaximumPacketSize = 39,
    WildcardSubscriptionAvailable = 40,
    SubscriptionIdentifierAvailable = 41,
    SharedSubscriptionAvailable = 42,
}

impl TryFrom<u8> for PropertyType {
    type Error = Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        let property = match value {
            1 => PropertyType::PayloadFormatIndicator,
            2 => PropertyType::MessageExpiryInterval,
            3 => PropertyType::ContentType,
            8 => PropertyType::ResponseTopic,
            9 => PropertyType::CorrelationData,
            11 => PropertyType::SubscriptionIdentifier,
            17 => PropertyType::SessionExpiryInterval,
            18 => PropertyType::AssignedClientIdentifier,
            19 => PropertyType::ServerKeepAlive,
            21 => PropertyType::AuthenticationMethod,
            22 => PropertyType::AuthenticationData,
            23 => PropertyType::RequestProblemInformation,
            24 => PropertyType::WillDelayInterval,
            25 => PropertyType::RequestResponseInformation,
            26 => PropertyType::ResponseInformation,
            28 => PropertyType::ServerReference,
            31 => PropertyType::ReasonString,
            33 => PropertyType::ReceiveMaximum,
            34 => PropertyType::TopicAliasMaximum,
            35 => PropertyType::TopicAlias,
            36 => PropertyType::MaximumQos,
            37 => PropertyType::RetainAvailable,
            38 => PropertyType::UserProperty,
            39 => PropertyType::MaximumPacketSize,
            40 => PropertyType::WildcardSubscriptionAvailable,
            41 => PropertyType::SubscriptionIdentifierAvailable,
            42 => PropertyType::SharedSubscriptionAvailable,
            num => return Err(Error::InvalidPropertyType(num)),
        };

        Ok(property)
    }
}
