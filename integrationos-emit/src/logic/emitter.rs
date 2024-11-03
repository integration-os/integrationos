use crate::{domain::event::Event, server::AppState};
use axum::{extract::State, routing::post, Json, Router};
use integrationos_domain::{IntegrationOSError, Unit};
use std::sync::Arc;

pub fn get_router() -> Router<Arc<AppState>> {
    Router::new().route("/emit", post(emit))
}

pub async fn emit(
    state: State<Arc<AppState>>,
    Json(payload): Json<Event>,
) -> Result<Unit, IntegrationOSError> {
    tracing::info!("Received event: {:?}", payload);

    state.stream_client.publish(payload).await?;

    Ok(())
}
