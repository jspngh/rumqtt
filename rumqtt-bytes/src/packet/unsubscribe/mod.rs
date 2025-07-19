use crate::Properties;

pub(crate) mod v4;
pub(crate) mod v5;

/// Unsubscribe request
///
/// Sent by the client to the server to unsubscribe from topics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unsubscribe {
    pub pkid: u16,
    pub properties: Properties,
    pub filters: Vec<String>,
}

impl Unsubscribe {
    pub fn new<S: Into<String>>(topic: S) -> Unsubscribe {
        Unsubscribe {
            pkid: 0,
            filters: vec![topic.into()],
            properties: Properties::new(),
        }
    }

    pub fn new_many<F: IntoIterator<Item = String>>(
        filters: F,
        properties: Option<Properties>,
    ) -> Self {
        Self {
            pkid: 0,
            filters: filters.into_iter().collect(),
            properties: properties.unwrap_or_default(),
        }
    }
}
