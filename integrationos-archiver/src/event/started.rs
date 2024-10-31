use super::EventMetadata;
use chrono::{DateTime, Utc};
use integrationos_domain::{prefix::IdPrefix, Id};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Started {
    #[serde(rename = "_id")]
    id: Id,
    started_at: DateTime<Utc>,
    collection: String,
}

impl Started {
    pub fn new(store: String) -> Self {
        Self {
            id: Id::now(IdPrefix::Archive),
            started_at: Utc::now(),
            collection: store,
        }
    }

    pub fn started_at(&self) -> DateTime<Utc> {
        self.started_at
    }

    pub fn collection(&self) -> &str {
        &self.collection
    }
}

impl EventMetadata for Started {
    fn reference(&self) -> Id {
        self.id
    }
}
