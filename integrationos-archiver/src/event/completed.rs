use super::EventMetadata;
use chrono::{DateTime, NaiveDate, Utc};
use integrationos_domain::Id;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Completed {
    id: Id,
    path: String,
    completed_at: DateTime<Utc>,
}

impl Completed {
    pub fn new(path: String, id: Id) -> Self {
        Self {
            path,
            id,
            completed_at: Utc::now(),
        }
    }

    pub fn date(&self) -> NaiveDate {
        self.completed_at.date_naive()
    }
}

impl EventMetadata for Completed {
    fn reference(&self) -> Id {
        self.id
    }
}
