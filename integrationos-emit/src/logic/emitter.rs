use crate::{
    domain::{
        event::Event,
        idempotency::{Idempotency, IdempotencyKey},
    },
    middleware::idempotency::{header_idempotency, IDEMPOTENCY_HEADER_STR},
    server::AppState,
    stream::EventStreamTopic,
};
use axum::{extract::State, middleware::from_fn, routing::post, Extension, Json, Router};
use http::HeaderName;
use integrationos_domain::{
    prefix::IdPrefix, record_metadata::RecordMetadata, ApplicationError, Id, IntegrationOSError,
};
use mongodb::bson::doc;
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

#[tracing::instrument(skip(state, event))]
pub async fn emit(
    State(state): State<Arc<AppState>>,
    Extension(idempotency_key): Extension<IdempotencyKey>,
    Json(event): Json<Event>,
) -> Result<Json<Id>, IntegrationOSError> {
    let is_processed = state
        .app_stores
        .idempotency
        .get_one(doc! {
            "_id": idempotency_key.inner()
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
            indexable: Id::now(IdPrefix::Idempotency),
            metadata: RecordMetadata::default(),
        };

        state
            .app_stores
            .idempotency
            .create_one(&idempotency)
            .await?;

        let id = state
            .event_stream
            .publish(event.as_entity(), EventStreamTopic::Target)
            .await?;

        Ok(Json(id))
    }
}
