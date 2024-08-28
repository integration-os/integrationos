use super::EventMetadata;
use chrono::{DateTime, NaiveDate, Utc};
use integrationos_domain::Id;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Uploaded {
    id: Id,
    uploaded_at: DateTime<Utc>,
}

impl Uploaded {
    pub fn new(id: Id) -> Self {
        Self {
            id,
            uploaded_at: Utc::now(),
        }
    }

    pub fn date(&self) -> NaiveDate {
        self.uploaded_at.date_naive()
    }
}

impl EventMetadata for Uploaded {
    fn reference(&self) -> Id {
        self.id
    }
}
