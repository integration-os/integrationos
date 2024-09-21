use super::EventMetadata;
use chrono::{DateTime, Utc};
use integrationos_domain::Id;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Failed {
    id: Id,
    failed_at: DateTime<Utc>,
    start_time: DateTime<Utc>,
    end_time: DateTime<Utc>,
    reason: String,
}

impl Failed {
    pub fn new(reason: String, id: Id, start_time: DateTime<Utc>, end_time: DateTime<Utc>) -> Self {
        Self {
            id,
            reason,
            start_time,
            end_time,
            failed_at: Utc::now(),
        }
    }
}

impl EventMetadata for Failed {
    fn reference(&self) -> Id {
        self.id
    }
}
