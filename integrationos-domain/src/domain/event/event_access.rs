use serde::{Deserialize, Serialize};

use crate::{
    id::Id,
    prelude::{
        configuration::environment::Environment,
        connection::connection_definition::{ConnectionDefinitionType, Paths},
        shared::{ownership::Ownership, record_metadata::RecordMetadata},
    },
};

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "dummy", derive(fake::Dummy))]
#[serde(rename_all = "camelCase")]
pub struct EventAccess {
    #[serde(rename = "_id")]
    pub id: Id,
    pub name: String,
    pub key: String,
    pub namespace: String,
    pub platform: String,
    pub r#type: ConnectionDefinitionType,
    pub group: String,
    pub ownership: Ownership,
    pub paths: Paths,
    #[cfg_attr(feature = "dummy", dummy(faker = "8..50"))]
    pub access_key: String,
    #[serde(default = "throughput_default")]
    pub throughput: u64,
    pub environment: Environment,
    #[serde(flatten, default)]
    pub record_metadata: RecordMetadata,
}

fn throughput_default() -> u64 {
    500
}

impl EventAccess {
    pub fn with_key(mut self, key: String) -> Self {
        self.key = key;
        self
    }
}
