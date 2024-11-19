use crate::server::AppState;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use http::header::AUTHORIZATION;
use integrationos_domain::{
    prefix::IdPrefix, record_metadata::RecordMetadata, ApplicationError, Claims, Id,
    IntegrationOSError, InternalError, Unit,
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
        schedule_on: Option<i64>,
    },
}

#[async_trait]
impl EventExt for Event {
    async fn side_effect(&self, ctx: &AppState, entity_id: Id) -> Result<Unit, IntegrationOSError> {
        match self {
            Event::DatabaseConnectionLost { connection_id, .. } => {
                tracing::info!("Processed event for connection {connection_id}");
                Ok(())
                // let base_path = &ctx.config.event_callback_url;
                // let path = format!("{base_path}/database-connection-lost/{connection_id}");
                //
                // let authorization = Claims::from_secret(ctx.config.jwt_secret.as_str())?;
                //
                // ctx.http_client
                //     .post(path)
                //     .header(AUTHORIZATION, format!("Bearer {authorization}"))
                //     .send()
                //     .await
                //     .inspect(|res| {
                //         tracing::info!("Response: {:?}", res);
                //     })
                //     .map_err(|e| {
                //         tracing::error!("Failed to build request for entity id {entity_id}: {e}");
                //         InternalError::io_err(
                //             &format!("Failed to build request for entity id {entity_id}"),
                //             None,
                //         )
                //     })?
                //     .error_for_status()
                //     .map_err(|e| {
                //         tracing::error!("Failed to execute request for entity id {entity_id}: {e}");
                //         ApplicationError::bad_request(
                //             &format!("Failed to execute request for entity id {entity_id}"),
                //             None,
                //         )
                //     })
                //     .map(|res| tracing::info!("Response: {:?}", res))
            }
        }
    }
}

impl Event {
    pub fn as_entity(&self) -> EventEntity {
        EventEntity {
            entity: self.clone(),
            entity_id: Id::now(IdPrefix::PipelineEvent),
            outcome: EventOutcome::Created,
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
    pub outcome: EventOutcome,
    #[serde(flatten, default)]
    pub metadata: RecordMetadata,
}

impl EventEntity {
    pub fn with_outcome(&self, outcome: EventOutcome) -> Self {
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
        self.outcome.err()
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, EnumString, AsRefStr)]
#[serde(rename_all = "kebab-case", tag = "type")]
#[strum(serialize_all = "kebab-case")]
pub enum EventOutcome {
    Created,
    Executed { timestamp: i64 },
    Succeded { retries: u32 },
    Errored { error: String, retries: u32 },
}

impl EventOutcome {
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

    fn err(&self) -> Option<String> {
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
