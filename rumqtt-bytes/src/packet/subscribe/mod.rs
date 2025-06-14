use bytes::{Buf, BufMut, Bytes, BytesMut};

use crate::{parse::*, property::PropertyType, Error, QoS};

pub(crate) mod v4;
pub(crate) mod v5;

/// Subscribe request
///
/// Sent from the client to the server to create one or more subscriptions.
/// Each subscription registers a client’s interest in one or more topics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Subscribe {
    pub pkid: u16,
    pub properties: Option<SubscribeProperties>,
    pub filters: Vec<Filter>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubscribeProperties {
    pub subscription_id: Option<VarInt>,
    pub user_properties: Vec<(String, String)>,
}

/// Subscription filter
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Filter {
    pub path: String,
    pub qos: QoS,
    pub nolocal: bool,
    pub preserve_retain: bool,
    pub retain_forward_rule: RetainForwardRule,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RetainForwardRule {
    OnEverySubscribe,
    OnNewSubscribe,
    Never,
}

impl Subscribe {
    pub fn new(filter: Filter, properties: Option<SubscribeProperties>) -> Self {
        Self {
            pkid: 0,
            filters: vec![filter],
            properties,
        }
    }

    pub fn from_string<S: Into<String>>(path: S, qos: QoS) -> Subscribe {
        let filter = Filter {
            path: path.into(),
            qos,
            nolocal: false,
            preserve_retain: false,
            retain_forward_rule: RetainForwardRule::OnEverySubscribe,
        };

        Subscribe {
            pkid: 0,
            filters: vec![filter],
            properties: None,
        }
    }

    pub fn new_many<F>(filters: F, properties: Option<SubscribeProperties>) -> Self
    where
        F: IntoIterator<Item = Filter>,
    {
        Self {
            pkid: 0,
            filters: filters.into_iter().collect(),
            properties,
        }
    }
}

impl SubscribeProperties {
    fn len(&self) -> Result<VarInt, Error> {
        let mut len = 0;

        if let Some(id) = &self.subscription_id {
            len += 1 + id.length();
        }

        for (key, value) in self.user_properties.iter() {
            len += 1 + 2 + key.len() + 2 + value.len();
        }

        VarInt::new(len)
    }

    pub fn read(bytes: &mut Bytes) -> Result<Option<SubscribeProperties>, Error> {
        let mut id = None;
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
                PropertyType::SubscriptionIdentifier => {
                    let sub_id = VarInt::read(bytes.iter())?;
                    if sub_id.value() == 0 {
                        return Err(Error::SubscriptionIdZero);
                    }

                    cursor += sub_id.length();
                    bytes.advance(sub_id.length());
                    id = Some(sub_id);
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

        Ok(Some(SubscribeProperties {
            subscription_id: id,
            user_properties,
        }))
    }

    pub fn write(&self, buffer: &mut BytesMut) -> Result<(), Error> {
        let len = self.len()?;
        len.write(buffer);

        if let Some(id) = &self.subscription_id {
            if id.value() == 0 {
                return Err(Error::SubscriptionIdZero);
            }

            buffer.put_u8(PropertyType::SubscriptionIdentifier as u8);
            id.write(buffer);
        }

        for (key, value) in self.user_properties.iter() {
            buffer.put_u8(PropertyType::UserProperty as u8);
            write_mqtt_string(buffer, key);
            write_mqtt_string(buffer, value);
        }

        Ok(())
    }
}

impl Filter {
    pub fn new(path: String, qos: QoS) -> Self {
        Self {
            path,
            qos,
            nolocal: false,
            preserve_retain: false,
            retain_forward_rule: RetainForwardRule::OnEverySubscribe,
        }
    }

    pub fn read(bytes: &mut Bytes) -> Result<Vec<Filter>, Error> {
        // variable header size = 2 (packet identifier)
        let mut filters = Vec::new();

        while bytes.has_remaining() {
            let path = read_mqtt_string(bytes)?;
            let options = read_u8(bytes)?;
            let requested_qos = options & 0b0000_0011;

            let nolocal = (options >> 2) & 0b0000_0001;
            let nolocal = nolocal != 0;

            let preserve_retain = (options >> 3) & 0b0000_0001;
            let preserve_retain = preserve_retain != 0;

            let retain_forward_rule = (options >> 4) & 0b0000_0011;
            let retain_forward_rule = match retain_forward_rule {
                0 => RetainForwardRule::OnEverySubscribe,
                1 => RetainForwardRule::OnNewSubscribe,
                2 => RetainForwardRule::Never,
                r => return Err(Error::InvalidRetainForwardRule(r)),
            };

            filters.push(Filter {
                path,
                qos: requested_qos.try_into()?,
                nolocal,
                preserve_retain,
                retain_forward_rule,
            });
        }

        Ok(filters)
    }

    pub fn write(&self, buffer: &mut BytesMut) {
        let mut options = 0;
        options |= self.qos as u8;

        if self.nolocal {
            options |= 0b0000_0100;
        }

        if self.preserve_retain {
            options |= 0b0000_1000;
        }

        options |= match self.retain_forward_rule {
            RetainForwardRule::OnEverySubscribe => 0b0000_0000,
            RetainForwardRule::OnNewSubscribe => 0b0001_0000,
            RetainForwardRule::Never => 0b0010_0000,
        };

        write_mqtt_string(buffer, self.path.as_str());
        buffer.put_u8(options);
    }

    fn len(&self) -> usize {
        // filter len + filter + options
        2 + self.path.len() + 1
    }
}
