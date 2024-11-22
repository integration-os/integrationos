use crate::{algebra::event::EventExt, server::AppState};
use chrono::Utc;
use integrationos_domain::{
    prefix::IdPrefix, record_metadata::RecordMetadata, Id, IntegrationOSError, Unit,
};
use serde::{Deserialize, Serialize};
use strum::{AsRefStr, EnumString};

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase", tag = "type")]
pub enum Event {
    #[serde(rename_all = "camelCase")]
    DatabaseConnectionLost {
        connection_id: Id,
        schedule_on: Option<i64>,
    },
}

impl Event {
    pub fn as_entity(&self) -> EventEntity {
        EventEntity {
            entity: self.clone(),
            entity_id: Id::now(IdPrefix::PipelineEvent),
            outcome: EventStatus::Created,
            metadata: RecordMetadata::default(),
        }
    }

    pub fn scheduled_on(&self) -> Option<i64> {
        match self {
            Event::DatabaseConnectionLost { schedule_on, .. } => *schedule_on,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EventEntity {
    #[serde(rename = "_id")]
    pub entity_id: Id,
    pub entity: Event,
    pub outcome: EventStatus,
    #[serde(flatten, default)]
    pub metadata: RecordMetadata,
}

impl EventEntity {
    pub fn with_outcome(&self, outcome: EventStatus) -> Self {
        let mut metadata = self.metadata.clone();
        metadata.mark_updated("system");
        Self {
            entity_id: self.entity_id,
            entity: self.entity.clone(),
            outcome,
            metadata,
        }
    }

    pub async fn side_effect(&self, ctx: &AppState) -> Result<Unit, IntegrationOSError> {
        self.entity.side_effect(ctx, self.entity_id).await
    }

    pub fn retries(&self) -> u32 {
        self.outcome.retries()
    }

    pub fn error(&self) -> Option<String> {
        self.outcome.error()
    }

    pub fn is_created(&self) -> bool {
        matches!(self.outcome, EventStatus::Created)
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, EnumString, AsRefStr)]
#[serde(rename_all = "kebab-case", tag = "type")]
#[strum(serialize_all = "kebab-case")]
pub enum EventStatus {
    Created,
    Executed { timestamp: i64 },
    Succeded { retries: u32 },
    Errored { error: String, retries: u32 },
}

impl EventStatus {
    pub fn succeded(retries: u32) -> Self {
        Self::Succeded { retries }
    }

    pub fn errored(error: String, retries: u32) -> Self {
        Self::Errored { error, retries }
    }

    pub fn executed() -> Self {
        Self::Executed {
            timestamp: Utc::now().timestamp_millis(),
        }
    }

    fn retries(&self) -> u32 {
        match self {
            Self::Errored { retries, .. } => *retries,
            Self::Succeded { retries, .. } => *retries,
            _ => 0,
        }
    }

    fn error(&self) -> Option<String> {
        match self {
            Self::Errored { error, .. } => Some(error.clone()),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ScheduledEvent {
    #[serde(rename = "_id")]
    pub id: Id,
    pub event: EventEntity,
    pub schedule_on: i64,
}
