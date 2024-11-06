use crate::server::AppState;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use integrationos_domain::{
    prefix::IdPrefix, record_metadata::RecordMetadata, Id, IntegrationOSError, Unit,
};
use serde::{Deserialize, Serialize};
use strum::{AsRefStr, EnumString};

#[async_trait]
pub trait EventExt {
    async fn side_effect(&self, ctx: &AppState, entity_id: Id) -> Result<Unit, IntegrationOSError>;
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase", tag = "type")]
pub enum Event {
    #[serde(rename_all = "camelCase")]
    DatabaseConnectionLost {
        connection_id: Id,
        schedule_on: DateTime<Utc>,
    },
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
            parent_id: None,
            outcome: None,
            metadata: RecordMetadata::default(),
        }
    }

    fn execute_now(&self) -> bool {
        match self {
            Event::DatabaseConnectionLost { schedule_on, .. } => schedule_on.lt(&Utc::now()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EventEntity {
    #[serde(rename = "_id")]
    pub entity_id: Id,
    pub parent_id: Option<Id>,
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

    pub fn execute_now(&self) -> bool {
        self.entity.execute_now()
    }

    /**
     * Get a new event with the same data but with a new id, linked to the current event by the parent_id
     *
     * As events are immutable, we need to create a new one with the same data but with a new id
     * This is used to create a new event with the same data but with a new id
     */
    pub fn new_immutable(&self) -> Self {
        Self {
            entity_id: Id::now(IdPrefix::PipelineEvent),
            parent_id: Some(self.entity_id),
            entity: self.entity.clone(),
            outcome: None,
            metadata: RecordMetadata::default(),
        }
    }

    pub fn retries(&self) -> u32 {
        self.outcome.iter().map(|o| o.retries()).sum()
    }

    pub fn error(&self) -> Option<String> {
        self.outcome.iter().filter_map(|o| o.err()).next()
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, EnumString, AsRefStr)]
#[serde(rename_all = "kebab-case", tag = "type")]
#[strum(serialize_all = "kebab-case")]
pub enum EventOutcome {
    Success,
    Error { error: String, retries: u32 },
    Deferred,
}

impl EventOutcome {
    pub fn success() -> Self {
        Self::Success
    }

    pub fn error(error: String, retries: u32) -> Self {
        Self::Error { error, retries }
    }

    pub fn deferred() -> Self {
        Self::Deferred
    }

    fn retries(&self) -> u32 {
        match self {
            Self::Error { retries, .. } => *retries,
            _ => 0,
        }
    }

    fn err(&self) -> Option<String> {
        match self {
            Self::Error { error, .. } => Some(error.clone()),
            _ => None,
        }
    }
}
