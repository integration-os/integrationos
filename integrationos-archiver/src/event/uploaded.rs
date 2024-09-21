use super::EventMetadata;
use chrono::{DateTime, Utc};
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
}

impl EventMetadata for Uploaded {
    fn reference(&self) -> Id {
        self.id
    }
}
