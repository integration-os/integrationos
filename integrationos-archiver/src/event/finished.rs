use super::EventMetadata;
use anyhow::Result;
use chrono::{DateTime, Utc};
use integrationos_domain::Id;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Finished {
    id: Id,
    finished_at: DateTime<Utc>,
}

impl Finished {
    pub fn new(id: Id) -> Result<Self> {
        Ok(Self {
            id,
            finished_at: Utc::now(),
        })
    }
}

impl EventMetadata for Finished {
    fn reference(&self) -> Id {
        self.id
    }
}
