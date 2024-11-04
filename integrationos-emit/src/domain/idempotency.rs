use integrationos_domain::{record_metadata::RecordMetadata, Id};
use serde::{Deserialize, Serialize};
use std::fmt::Display;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct IdempotencyKey(String);

impl IdempotencyKey {
    pub fn new(key: String) -> Self {
        Self(key)
    }

    pub fn inner(&self) -> &str {
        &self.0
    }
}

impl Display for IdempotencyKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

// TODO: Add a TTL to the key and create index
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Idempotency {
    #[serde(rename = "_id")]
    pub id: Id,
    pub key: IdempotencyKey,
    pub metadata: RecordMetadata,
}
