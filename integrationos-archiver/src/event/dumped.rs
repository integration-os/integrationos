use super::EventMetadata;
use chrono::{DateTime, NaiveDate, Utc};
use integrationos_domain::Id;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Dumped {
    id: Id,
    dumped_at: DateTime<Utc>,
}

impl Dumped {
    pub fn new(id: Id) -> Self {
        Self {
            id,
            dumped_at: Utc::now(),
        }
    }

    pub fn date(&self) -> NaiveDate {
        self.dumped_at.date_naive()
    }
}

impl EventMetadata for Dumped {
    fn reference(&self) -> Id {
        self.id
    }
}
