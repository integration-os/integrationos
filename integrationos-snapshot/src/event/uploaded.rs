use super::EventMetadata;
use chrono::{DateTime, Utc};
use integrationos_domain::Id;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Uploaded {
    id: Id,
    uploaded_at: DateTime<Utc>,
    path: String,
}

impl Uploaded {
    pub fn new(path: String, id: Id) -> Self {
        Self {
            id,
            path,
            uploaded_at: Utc::now(),
        }
    }
}

impl EventMetadata for Uploaded {
    fn reference(&self) -> Id {
        self.id
    }
}
