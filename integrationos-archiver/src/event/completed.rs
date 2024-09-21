use super::EventMetadata;
use chrono::{DateTime, Utc};
use integrationos_domain::Id;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Completed {
    id: Id,
    path: String,
    completed_at: DateTime<Utc>,
    start_time: DateTime<Utc>,
    end_time: DateTime<Utc>,
}

impl Completed {
    pub fn new(path: String, id: Id, start_time: DateTime<Utc>, end_time: DateTime<Utc>) -> Self {
        Self {
            path,
            id,
            completed_at: Utc::now(),
            start_time,
            end_time,
        }
    }
}

impl EventMetadata for Completed {
    fn reference(&self) -> Id {
        self.id
    }
}
