use super::EventMetadata;
use chrono::{DateTime, NaiveDate, Utc};
use integrationos_domain::Id;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Failed {
    id: Id,
    failed_at: DateTime<Utc>,
    reason: String,
}

impl Failed {
    pub fn new(reason: String, id: Id) -> Self {
        Self {
            id,
            reason,
            failed_at: Utc::now(),
        }
    }

    pub fn date(&self) -> NaiveDate {
        self.failed_at.date_naive()
    }
}

impl EventMetadata for Failed {
    fn reference(&self) -> Id {
        self.id
    }
}
