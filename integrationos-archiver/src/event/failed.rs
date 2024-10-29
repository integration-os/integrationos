use super::EventMetadata;
use chrono::{DateTime, Utc};
use integrationos_domain::{prefix::IdPrefix, Id};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Failed {
    #[serde(rename = "_id")]
    id: Id,
    reference: Id,
    failed_at: DateTime<Utc>,
    start_time: i64,
    end_time: i64,
    reason: String,
}

impl Failed {
    pub fn new(reason: String, id: Id, start_time: DateTime<Utc>, end_time: DateTime<Utc>) -> Self {
        Self {
            id: Id::now(IdPrefix::Archive),
            reference: id,
            reason,
            start_time: start_time.timestamp_millis(),
            end_time: end_time.timestamp_millis(),
            failed_at: Utc::now(),
        }
    }
}

impl EventMetadata for Failed {
    fn reference(&self) -> Id {
        self.reference
    }
}
