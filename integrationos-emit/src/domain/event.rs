use integrationos_domain::{prefix::IdPrefix, record_metadata::RecordMetadata, Id};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase", tag = "type")]
pub enum Event {
    DatabaseConnectionLost {
        connection_id: Id,
        metadata: RecordMetadata,
    },
}

impl Event {
    pub fn entity_id(&self) -> Id {
        match self {
            Event::DatabaseConnectionLost { .. } => Id::now(IdPrefix::PipelineEvent),
        }
    }

    pub fn partition_key(&self) -> String {
        match self {
            Event::DatabaseConnectionLost { .. } => "connection-broken".to_string(),
        }
    }
}
