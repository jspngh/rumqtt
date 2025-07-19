//! Module for working with MQTT properties

use bytes::{Buf, BufMut, Bytes, BytesMut};

use crate::{Error, VarInt};

/// MQTT 5.0 Property
///
/// This is a key-value pair that can be included in the variable header of packets.
/// A property consists of an identifier (encoded as a Variable Byte Integer) followed by a value.
///
/// This enumeration list all the possible properties that can be used in MQTT 5.0.
/// However, not all properties can be used in every packet.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(clippy::enum_variant_names)]
pub enum Property {
    PayloadFormatIndicator(u8),
    MessageExpiryInterval(u32),
    ContentType(String),
    ResponseTopic(String),
    CorrelationData(Bytes),
    SubscriptionIdentifier(VarInt),
    SessionExpiryInterval(u32),
    AssignedClientIdentifier(String),
    ServerKeepAlive(u16),
    AuthenticationMethod(String),
    AuthenticationData(Bytes),
    RequestProblemInformation(bool),
    WillDelayInterval(u32),
    RequestResponseInformation(bool),
    ResponseInformation(String),
    ServerReference(String),
    ReasonString(String),
    ReceiveMaximum(u16),
    TopicAliasMaximum(u16),
    TopicAlias(u16),
    MaximumQos(u8),
    RetainAvailable(bool),
    UserProperty { name: String, value: String },
    MaximumPacketSize(u32),
    WildcardSubscriptionAvailable(bool),
    SubscriptionIdentifierAvailable(bool),
    SharedSubscriptionAvailable(bool),
}

impl Property {
    fn len(&self) -> usize {
        match self {
            Property::PayloadFormatIndicator(_) => 1,
            Property::MessageExpiryInterval(_) => 4,
            Property::ContentType(s) => 2 + s.len(),
            Property::ResponseTopic(s) => 2 + s.len(),
            Property::CorrelationData(v) => 2 + v.len(),
            Property::SubscriptionIdentifier(varint) => varint.length(),
            Property::SessionExpiryInterval(_) => 4,
            Property::AssignedClientIdentifier(s) => 2 + s.len(),
            Property::ServerKeepAlive(_) => 2,
            Property::AuthenticationMethod(s) => 2 + s.len(),
            Property::AuthenticationData(v) => 2 + v.len(),
            Property::RequestProblemInformation(_) => 1,
            Property::WillDelayInterval(_) => 4,
            Property::RequestResponseInformation(_) => 1,
            Property::ResponseInformation(s) => 2 + s.len(),
            Property::ServerReference(s) => 2 + s.len(),
            Property::ReasonString(s) => 2 + s.len(),
            Property::ReceiveMaximum(_) => 2,
            Property::TopicAliasMaximum(_) => 2,
            Property::TopicAlias(_) => 2,
            Property::MaximumQos(_) => 1,
            Property::RetainAvailable(_) => 1,
            Property::UserProperty { name, value } => 2 + name.len() + 2 + value.len(),
            Property::MaximumPacketSize(_) => 4,
            Property::WildcardSubscriptionAvailable(_) => 1,
            Property::SubscriptionIdentifierAvailable(_) => 1,
            Property::SharedSubscriptionAvailable(_) => 1,
        }
    }
}

/// A set of [Property] values, contained in the variable header of a packet.
///
/// This abstracts over the underlying property collection, only exposing an interface
/// that allows for extending the set and for iteration over the individual properties.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Properties(Vec<Property>);

/// Create new [Properties] from a given list of [Property] instances.
///
/// # Example
/// ```
/// # use rumqtt_bytes::{properties, Property};
/// let props = properties![
///     Property::ReceiveMaximum(20),
///     Property::MaximumPacketSize(1024),
/// ];
/// ```
#[macro_export]
macro_rules! properties {
    () => (
        $crate::Properties::new()
    );
    ($($x:expr),+ $(,)?) => (
        $crate::Properties::from_vec(vec![$($x),+])
    );
}

impl Properties {
    /// Create a new empty [Properties] instance.
    pub const fn new() -> Self {
        Properties(Vec::new())
    }

    /// Create a new [Properties] instance containing a list of [Property]'s.
    pub const fn from_vec(properties: Vec<Property>) -> Self {
        Properties(properties)
    }

    /// Add a property to the list of properties.
    pub fn add(&mut self, property: Property) {
        self.0.push(property);
    }

    pub(crate) fn read(stream: &mut Bytes, allow_list: &[PropertyType]) -> Result<Self, Error> {
        let mut properties = Vec::new();
        let properties_len = crate::VarInt::read(stream.iter())?;
        stream.advance(properties_len.length());

        let mut cursor = 0;
        while properties_len > cursor {
            let property_type = PropertyType::try_from(crate::parse::read_u8(stream)?)?;
            if !allow_list.contains(&property_type) {
                return Err(Error::InvalidPropertyType(property_type as u8));
            }

            let property = match property_type {
                PropertyType::PayloadFormatIndicator => {
                    Property::PayloadFormatIndicator(crate::parse::read_u8(stream)?)
                }
                PropertyType::MessageExpiryInterval => {
                    Property::MessageExpiryInterval(crate::parse::read_u32(stream)?)
                }
                PropertyType::ContentType => {
                    Property::ContentType(crate::parse::read_mqtt_string(stream)?)
                }
                PropertyType::ResponseTopic => {
                    Property::ResponseTopic(crate::parse::read_mqtt_string(stream)?)
                }
                PropertyType::CorrelationData => {
                    Property::CorrelationData(crate::parse::read_mqtt_bytes(stream)?)
                }
                PropertyType::SubscriptionIdentifier => {
                    let varint = crate::VarInt::read(stream.iter())?;
                    stream.advance(varint.length());
                    Property::SubscriptionIdentifier(varint)
                }
                PropertyType::SessionExpiryInterval => {
                    Property::SessionExpiryInterval(crate::parse::read_u32(stream)?)
                }
                PropertyType::AssignedClientIdentifier => {
                    Property::AssignedClientIdentifier(crate::parse::read_mqtt_string(stream)?)
                }
                PropertyType::ServerKeepAlive => {
                    Property::ServerKeepAlive(crate::parse::read_u16(stream)?)
                }
                PropertyType::AuthenticationMethod => {
                    Property::AuthenticationMethod(crate::parse::read_mqtt_string(stream)?)
                }
                PropertyType::AuthenticationData => {
                    Property::AuthenticationData(crate::parse::read_mqtt_bytes(stream)?)
                }
                PropertyType::RequestProblemInformation => {
                    Property::RequestProblemInformation(crate::parse::read_u8(stream)? != 0)
                }
                PropertyType::WillDelayInterval => {
                    Property::WillDelayInterval(crate::parse::read_u32(stream)?)
                }
                PropertyType::RequestResponseInformation => {
                    Property::RequestResponseInformation(crate::parse::read_u8(stream)? != 0)
                }
                PropertyType::ResponseInformation => {
                    Property::ResponseInformation(crate::parse::read_mqtt_string(stream)?)
                }
                PropertyType::ServerReference => {
                    Property::ServerReference(crate::parse::read_mqtt_string(stream)?)
                }
                PropertyType::ReasonString => {
                    Property::ReasonString(crate::parse::read_mqtt_string(stream)?)
                }
                PropertyType::ReceiveMaximum => {
                    Property::ReceiveMaximum(crate::parse::read_u16(stream)?)
                }
                PropertyType::TopicAliasMaximum => {
                    Property::TopicAliasMaximum(crate::parse::read_u16(stream)?)
                }
                PropertyType::TopicAlias => Property::TopicAlias(crate::parse::read_u16(stream)?),
                PropertyType::MaximumQos => Property::MaximumQos(crate::parse::read_u8(stream)?),
                PropertyType::RetainAvailable => {
                    Property::RetainAvailable(crate::parse::read_u8(stream)? != 0)
                }
                PropertyType::UserProperty => {
                    let name = crate::parse::read_mqtt_string(stream)?;
                    let value = crate::parse::read_mqtt_string(stream)?;
                    Property::UserProperty { name, value }
                }
                PropertyType::MaximumPacketSize => {
                    Property::MaximumPacketSize(crate::parse::read_u32(stream)?)
                }
                PropertyType::WildcardSubscriptionAvailable => {
                    Property::WildcardSubscriptionAvailable(crate::parse::read_u8(stream)? != 0)
                }
                PropertyType::SubscriptionIdentifierAvailable => {
                    Property::SubscriptionIdentifierAvailable(crate::parse::read_u8(stream)? != 0)
                }
                PropertyType::SharedSubscriptionAvailable => {
                    Property::SharedSubscriptionAvailable(crate::parse::read_u8(stream)? != 0)
                }
            };

            cursor += 1 + property.len();
            properties.push(property);
        }

        Ok(Properties(properties))
    }

    pub(crate) fn write(&self, stream: &mut BytesMut) -> Result<usize, Error> {
        let varint = self.len()?;
        varint.write(stream);

        for property in &self.0 {
            match property {
                Property::PayloadFormatIndicator(value) => {
                    stream.put_u8(PropertyType::PayloadFormatIndicator as u8);
                    stream.put_u8(*value);
                }
                Property::MessageExpiryInterval(value) => {
                    stream.put_u8(PropertyType::MessageExpiryInterval as u8);
                    stream.put_u32(*value);
                }
                Property::ContentType(value) => {
                    stream.put_u8(PropertyType::ContentType as u8);
                    crate::parse::write_mqtt_string(stream, value);
                }
                Property::ResponseTopic(value) => {
                    stream.put_u8(PropertyType::ResponseTopic as u8);
                    crate::parse::write_mqtt_string(stream, value);
                }
                Property::CorrelationData(value) => {
                    stream.put_u8(PropertyType::CorrelationData as u8);
                    crate::parse::write_mqtt_bytes(stream, value);
                }
                Property::SubscriptionIdentifier(value) => {
                    stream.put_u8(PropertyType::SubscriptionIdentifier as u8);
                    value.write(stream);
                }
                Property::SessionExpiryInterval(value) => {
                    stream.put_u8(PropertyType::SessionExpiryInterval as u8);
                    stream.put_u32(*value);
                }
                Property::AssignedClientIdentifier(value) => {
                    stream.put_u8(PropertyType::AssignedClientIdentifier as u8);
                    crate::parse::write_mqtt_string(stream, value);
                }
                Property::ServerKeepAlive(value) => {
                    stream.put_u8(PropertyType::ServerKeepAlive as u8);
                    stream.put_u16(*value);
                }
                Property::AuthenticationMethod(value) => {
                    stream.put_u8(PropertyType::AuthenticationMethod as u8);
                    crate::parse::write_mqtt_string(stream, value);
                }
                Property::AuthenticationData(value) => {
                    stream.put_u8(PropertyType::AuthenticationData as u8);
                    crate::parse::write_mqtt_bytes(stream, value);
                }
                Property::RequestProblemInformation(value) => {
                    stream.put_u8(PropertyType::RequestProblemInformation as u8);
                    stream.put_u8(if *value { 1 } else { 0 });
                }
                Property::WillDelayInterval(value) => {
                    stream.put_u8(PropertyType::WillDelayInterval as u8);
                    stream.put_u32(*value);
                }
                Property::RequestResponseInformation(value) => {
                    stream.put_u8(PropertyType::RequestResponseInformation as u8);
                    stream.put_u8(if *value { 1 } else { 0 });
                }
                Property::ResponseInformation(value) => {
                    stream.put_u8(PropertyType::ResponseInformation as u8);
                    crate::parse::write_mqtt_string(stream, value);
                }
                Property::ServerReference(value) => {
                    stream.put_u8(PropertyType::ServerReference as u8);
                    crate::parse::write_mqtt_string(stream, value);
                }
                Property::ReasonString(value) => {
                    stream.put_u8(PropertyType::ReasonString as u8);
                    crate::parse::write_mqtt_string(stream, value);
                }
                Property::ReceiveMaximum(value) => {
                    stream.put_u8(PropertyType::ReceiveMaximum as u8);
                    stream.put_u16(*value);
                }
                Property::TopicAliasMaximum(value) => {
                    stream.put_u8(PropertyType::TopicAliasMaximum as u8);
                    stream.put_u16(*value);
                }
                Property::TopicAlias(value) => {
                    stream.put_u8(PropertyType::TopicAlias as u8);
                    stream.put_u16(*value);
                }
                Property::MaximumQos(value) => {
                    stream.put_u8(PropertyType::MaximumQos as u8);
                    stream.put_u8(*value);
                }
                Property::RetainAvailable(value) => {
                    stream.put_u8(PropertyType::RetainAvailable as u8);
                    stream.put_u8(if *value { 1 } else { 0 });
                }
                Property::UserProperty { name, value } => {
                    stream.put_u8(PropertyType::UserProperty as u8);
                    crate::parse::write_mqtt_string(stream, name);
                    crate::parse::write_mqtt_string(stream, value);
                }
                Property::MaximumPacketSize(value) => {
                    stream.put_u8(PropertyType::MaximumPacketSize as u8);
                    stream.put_u32(*value);
                }
                Property::WildcardSubscriptionAvailable(value) => {
                    stream.put_u8(PropertyType::WildcardSubscriptionAvailable as u8);
                    stream.put_u8(if *value { 1 } else { 0 });
                }
                Property::SubscriptionIdentifierAvailable(value) => {
                    stream.put_u8(PropertyType::SubscriptionIdentifierAvailable as u8);
                    stream.put_u8(if *value { 1 } else { 0 });
                }
                Property::SharedSubscriptionAvailable(value) => {
                    stream.put_u8(PropertyType::SharedSubscriptionAvailable as u8);
                    stream.put_u8(if *value { 1 } else { 0 });
                }
            }
        }

        Ok(1 + varint.length() + varint.value())
    }

    /// The size, as a variable byte integer, of the properties after serialization.
    pub(crate) fn len(&self) -> Result<crate::VarInt, Error> {
        let mut properties_len = 0;
        for property in &self.0 {
            // property id + property length
            properties_len += 1 + property.len();
        }

        VarInt::new(properties_len)
    }
}

impl core::ops::Deref for Properties {
    type Target = [Property];

    fn deref(&self) -> &Self::Target {
        self.0.as_slice()
    }
}

impl IntoIterator for Properties {
    type Item = Property;
    type IntoIter = std::vec::IntoIter<Property>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a> IntoIterator for &'a Properties {
    type Item = &'a Property;
    type IntoIter = std::slice::Iter<'a, Property>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl<'a> IntoIterator for &'a mut Properties {
    type Item = &'a mut Property;
    type IntoIter = std::slice::IterMut<'a, Property>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter_mut()
    }
}

/// Identifiers of the different properties used in MQTT 5.0
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PropertyType {
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
