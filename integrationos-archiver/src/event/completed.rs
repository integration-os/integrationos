use super::EventMetadata;
use chrono::{DateTime, Utc};
use integrationos_domain::{prefix::IdPrefix, Id};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Completed {
    #[serde(rename = "_id")]
    id: Id,
    reference: Id,
    path: String,
    completed_at: DateTime<Utc>,
    start_time: i64,
    end_time: i64,
}

impl Completed {
    pub fn new(path: String, id: Id, start_time: DateTime<Utc>, end_time: DateTime<Utc>) -> Self {
        Self {
            id: Id::now(IdPrefix::Archive),
            path,
            reference: id,
            completed_at: Utc::now(),
            start_time: start_time.timestamp_millis(),
            end_time: end_time.timestamp_millis(),
        }
    }
}

impl EventMetadata for Completed {
    fn reference(&self) -> Id {
        self.reference
    }
}
