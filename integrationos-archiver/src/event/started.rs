use std::str::FromStr;

use super::EventMetadata;
use anyhow::Result;
use chrono::{DateTime, Utc};
use integrationos_domain::{prefix::IdPrefix, Id, Store};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Started {
    #[serde(rename = "_id")]
    id: Id,
    started_at: DateTime<Utc>,
    collection: Store,
}

impl Started {
    pub fn new(collection: String) -> Result<Self> {
        let store = Store::from_str(&collection).map_err(|e| anyhow::anyhow!(e))?;
        Ok(Self {
            id: Id::now(IdPrefix::Archive),
            started_at: Utc::now(),
            collection: store,
        })
    }

    pub fn collection(&self) -> &Store {
        &self.collection
    }
}

impl EventMetadata for Started {
    fn reference(&self) -> Id {
        self.id
    }
}
