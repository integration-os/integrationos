use crate::server::AppState;
use async_trait::async_trait;
use integrationos_domain::{
    prefix::IdPrefix, record_metadata::RecordMetadata, Id, IntegrationOSError, Unit,
};
use serde::{Deserialize, Serialize};

#[async_trait]
pub trait EventExt {
    async fn side_effect(&self, ctx: &AppState, entity_id: Id) -> Result<Unit, IntegrationOSError>;
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase", tag = "type")]
pub enum Event {
    #[serde(rename_all = "camelCase")]
    DatabaseConnectionLost { connection_id: Id },
}

#[async_trait]
impl EventExt for Event {
    async fn side_effect(
        &self,
        _ctx: &AppState,
        entity_id: Id,
    ) -> Result<Unit, IntegrationOSError> {
        match self {
            Event::DatabaseConnectionLost { .. } => Ok(tracing::info!(
                "Received event: {:?}. With id: {entity_id}",
                self
            )),
        }
    }
}

impl Event {
    pub fn as_entity(&self) -> EventEntity {
        EventEntity {
            entity: self.clone(),
            entity_id: Id::now(IdPrefix::PipelineEvent),
            outcome: None,
            metadata: RecordMetadata::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EventEntity {
    #[serde(rename = "_id")]
    pub entity_id: Id,
    pub entity: Event,
    pub outcome: Option<EventOutcome>,
    #[serde(flatten)]
    pub metadata: RecordMetadata,
}

impl EventEntity {
    pub fn partition_key(&self) -> String {
        match self.entity {
            Event::DatabaseConnectionLost { .. } => "connection-broken".to_string(),
        }
    }

    pub async fn side_effect(&self, ctx: &AppState) -> Result<Unit, IntegrationOSError> {
        self.entity.side_effect(ctx, self.entity_id).await
    }

    pub fn outcome(mut self, outcome: EventOutcome) -> Self {
        self.outcome = Some(outcome);
        self
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EventOutcome {
    pub success: bool,
    pub retries: u32,
    pub error: Option<String>,
}
