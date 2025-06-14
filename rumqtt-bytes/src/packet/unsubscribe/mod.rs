use bytes::{Buf, BufMut, Bytes, BytesMut};

use crate::{parse::*, property::PropertyType, Error};

pub(crate) mod v4;
pub(crate) mod v5;

/// Unsubscribe request
///
/// Sent by the client to the server to unsubscribe from topics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unsubscribe {
    pub pkid: u16,
    pub properties: Option<UnsubscribeProperties>,
    pub filters: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnsubscribeProperties {
    pub user_properties: Vec<(String, String)>,
}

impl Unsubscribe {
    pub fn new<S: Into<String>>(topic: S) -> Unsubscribe {
        Unsubscribe {
            pkid: 0,
            filters: vec![topic.into()],
            properties: None,
        }
    }

    pub fn new_many<F: IntoIterator<Item = String>>(
        filters: F,
        properties: Option<UnsubscribeProperties>,
    ) -> Self {
        Self {
            pkid: 0,
            filters: filters.into_iter().collect(),
            properties,
        }
    }
}

impl UnsubscribeProperties {
    fn len(&self) -> Result<VarInt, Error> {
        let mut len = 0;

        for (key, value) in self.user_properties.iter() {
            len += 1 + 2 + key.len() + 2 + value.len();
        }

        VarInt::new(len)
    }

    pub fn read(bytes: &mut Bytes) -> Result<Option<UnsubscribeProperties>, Error> {
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
                PropertyType::UserProperty => {
                    let key = read_mqtt_string(bytes)?;
                    let value = read_mqtt_string(bytes)?;
                    cursor += 2 + key.len() + 2 + value.len();
                    user_properties.push((key, value));
                }
                _ => return Err(Error::InvalidPropertyType(prop)),
            }
        }

        Ok(Some(UnsubscribeProperties { user_properties }))
    }

    pub fn write(&self, buffer: &mut BytesMut) -> Result<(), Error> {
        let len = self.len()?;
        len.write(buffer);

        for (key, value) in self.user_properties.iter() {
            buffer.put_u8(PropertyType::UserProperty as u8);
            write_mqtt_string(buffer, key);
            write_mqtt_string(buffer, value);
        }

        Ok(())
    }
}
