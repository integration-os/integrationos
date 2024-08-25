use super::EventMetadata;
use chrono::{DateTime, Utc};
use integrationos_domain::Id;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
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
}

impl EventMetadata for Completed {
    fn reference(&self) -> Id {
        self.id
    }
}
