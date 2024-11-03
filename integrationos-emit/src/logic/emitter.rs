use crate::{domain::event::Event, server::AppState};
use axum::{extract::State, routing::post, Json, Router};
use integrationos_domain::{Id, IntegrationOSError};
use std::sync::Arc;

pub fn get_router() -> Router<Arc<AppState>> {
    Router::new().route("/emit", post(emit))
}

pub async fn emit(
    state: State<Arc<AppState>>,
    Json(payload): Json<Event>,
) -> Result<Json<Id>, IntegrationOSError> {
    tracing::info!("Received event: {:?}", payload);

    let id = state.stream_client.publish(payload.as_entity()).await?;

    Ok(Json(id))
}
