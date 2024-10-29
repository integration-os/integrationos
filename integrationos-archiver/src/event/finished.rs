use super::EventMetadata;
use chrono::{DateTime, Utc};
use integrationos_domain::{prefix::IdPrefix, Id};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Finished {
    #[serde(rename = "_id")]
    id: Id,
    reference: Id,
    finished_at: DateTime<Utc>,
}

impl Finished {
    pub fn new(id: Id) -> Self {
        Self {
            id: Id::now(IdPrefix::Archive),
            reference: id,
            finished_at: Utc::now(),
        }
    }
}

impl EventMetadata for Finished {
    fn reference(&self) -> Id {
        self.reference
    }
}
