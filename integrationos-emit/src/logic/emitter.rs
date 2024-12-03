use crate::{
    domain::{
        event::{Event, ScheduledEvent},
        idempotency::{Idempotency, IdempotencyKey},
    },
    middleware::idempotency::{header_idempotency, IDEMPOTENCY_HEADER_STR},
    server::AppState,
    stream::EventStreamTopic,
};
use axum::{extract::State, middleware::from_fn, routing::post, Extension, Json, Router};
use chrono::Utc;
use http::HeaderName;
use integrationos_domain::{
    prefix::IdPrefix, record_metadata::RecordMetadata, ApplicationError, Id, IntegrationOSError,
};
use mongodb::bson::doc;
use serde::{Deserialize, Serialize};
use std::{iter::once, sync::Arc};
use tower_http::sensitive_headers::SetSensitiveRequestHeadersLayer;

pub fn get_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/emit", post(emit))
        .layer(from_fn(header_idempotency))
        .layer(SetSensitiveRequestHeadersLayer::new(once(
            HeaderName::from_lowercase(IDEMPOTENCY_HEADER_STR.as_bytes()).unwrap(),
        )))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EntityIdResponse {
    pub idempotency_key: Id,
    pub entity_id: Id,
}

#[tracing::instrument(skip(state, event))]
pub async fn emit(
    State(state): State<Arc<AppState>>,
    Extension(idempotency_key): Extension<IdempotencyKey>,
    Json(event): Json<Event>,
) -> Result<Json<EntityIdResponse>, IntegrationOSError> {
    let is_processed = state
        .app_stores
        .idempotency
        .get_one(doc! {
            "_id": idempotency_key.inner().to_string()
        })
        .await
        .map(|idempotency| idempotency.is_some())
        .unwrap_or(false);

    if is_processed {
        Err(ApplicationError::conflict(
            &format!("Event with key {idempotency_key} already processed"),
            None,
        ))
    } else {
        let idempotency = Idempotency {
            key: idempotency_key.clone(),
            date: Utc::now(),
            metadata: RecordMetadata::default(),
        };

        state
            .app_stores
            .idempotency
            .create_one(&idempotency)
            .await?;

        match event.scheduled_on() {
            None => {
                let id = state
                    .event_stream
                    .publish(event.as_entity(), EventStreamTopic::Target)
                    .await?;

                Ok(Json(EntityIdResponse {
                    entity_id: id,
                    idempotency_key: idempotency_key.inner(),
                }))
            }
            Some(schedule_on) => {
                let scheduled = ScheduledEvent {
                    id: Id::now(IdPrefix::ScheduledEvent),
                    event: event.as_entity(),
                    schedule_on,
                };

                state
                    .app_stores
                    .scheduled
                    .create_one(&scheduled)
                    .await
                    .map_err(|e| anyhow::anyhow!("Failed to schedule event: {e}"))?;

                Ok(Json(EntityIdResponse {
                    entity_id: scheduled.id,
                    idempotency_key: idempotency_key.inner(),
                }))
            }
        }
    }
}
