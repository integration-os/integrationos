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
}

impl Started {
    pub fn new() -> Self {
        Self {
            id: Id::now(IdPrefix::Snapshot),
            started_at: Utc::now(),
        }
    }
}

impl EventMetadata for Started {
    fn reference(&self) -> Id {
        self.id
    }
}
