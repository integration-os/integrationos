use integrationos_domain::{prefix::IdPrefix, record_metadata::RecordMetadata, Id};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase", tag = "type")]
pub enum Event {
    #[serde(rename_all = "camelCase")]
    DatabaseConnectionLost { connection_id: Id },
}

impl Event {
    pub fn as_entity(&self) -> EventEntity {
        EventEntity {
            entity: self.clone(),
            entity_id: Id::now(IdPrefix::PipelineEvent),
            metadata: RecordMetadata::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct EventEntity {
    pub entity: Event,
    #[serde(rename = "_id")]
    pub entity_id: Id,
    pub metadata: RecordMetadata,
}

impl EventEntity {
    pub fn partition_key(&self) -> String {
        match self.entity {
            Event::DatabaseConnectionLost { .. } => "connection-broken".to_string(),
        }
    }
}
