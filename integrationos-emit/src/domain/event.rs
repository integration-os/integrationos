use crate::{router::generate_token, server::AppState};
use async_trait::async_trait;
use http::header::AUTHORIZATION;
use integrationos_domain::{
    prefix::IdPrefix, record_metadata::RecordMetadata, ApplicationError, Id, IntegrationOSError,
    InternalError, Unit,
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
                tracing::info!("Side effect for entity id {entity_id}");
                let base_path = &ctx.config.event_callback_url;
                let path = format!("{base_path}/database-connection-lost/{connection_id}");

                let authorization = generate_token(ctx)?;

                ctx.http_client
                    .post(path)
                    .header(AUTHORIZATION, format!("Bearer {authorization}"))
                    .send()
                    .await
                    .inspect(|res| {
                        tracing::info!("Response: {:?}", res);
                    })
                    .map_err(|e| {
                        tracing::error!("Failed to build request for entity id {entity_id}: {e}");
                        InternalError::io_err(
                            &format!("Failed to build request for entity id {entity_id}"),
                            None,
                        )
                    })?
                    .error_for_status()
                    .map_err(|e| {
                        tracing::error!("Failed to execute request for entity id {entity_id}: {e}");
                        ApplicationError::bad_request(
                            &format!("Failed to execute request for entity id {entity_id}"),
                            None,
                        )
                    })
                    .map(|res| tracing::info!("Response: {:?}", res))
            }
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
    pub outcome: Option<EventOutcome>,
    #[serde(flatten)]
    pub metadata: RecordMetadata,
}

impl EventEntity {
    pub fn with_outcome(&self, outcome: Option<EventOutcome>) -> Self {
        let mut metadata = self.metadata.clone();
        metadata.mark_updated("system");
        Self {
            entity_id: self.entity_id,
            entity: self.entity.clone(),
            outcome,
            metadata,
        }
    }

    pub fn partition_key(&self) -> String {
        match self.entity {
            Event::DatabaseConnectionLost { .. } => "connection-broken".to_string(),
        }
    }

    pub async fn side_effect(&self, ctx: &AppState) -> Result<Unit, IntegrationOSError> {
        self.entity.side_effect(ctx, self.entity_id).await
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
}

impl EventOutcome {
    pub fn success() -> Self {
        Self::Success
    }

    pub fn error(error: String, retries: u32) -> Self {
        Self::Error { error, retries }
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ScheduledEvent {
    #[serde(rename = "_id")]
    pub id: Id,
    pub event: EventEntity,
    pub schedule_on: i64,
}
