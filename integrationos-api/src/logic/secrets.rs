use crate::server::AppState;
use axum::{
    extract::{Path, State},
    routing::{get, post},
    Extension, Json, Router,
};
use bson::doc;
use integrationos_domain::{
    event_access::EventAccess, secret::Secret, ApplicationError, Id, IntegrationOSError,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;

pub fn get_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", post(create_secret))
        .route("/:id", get(get_secret))
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateSecretRequest {
    secret: Value,
}

async fn create_secret(
    state: State<Arc<AppState>>,
    Extension(event_access): Extension<Arc<EventAccess>>,
    Json(payload): Json<CreateSecretRequest>,
) -> Result<Json<Secret>, IntegrationOSError> {
    Ok(Json(
        state
            .secrets_client
            .create(&payload.secret, &event_access.ownership.id)
            .await?,
    ))
}

async fn get_secret(
    state: State<Arc<AppState>>,
    Extension(event_access): Extension<Arc<EventAccess>>,
    Path(id): Path<String>,
) -> Result<Json<Secret>, IntegrationOSError> {
    Ok(Json(
        state
            .secrets_client
            .get(&id, &event_access.ownership.id)
            .await?,
    ))
}

pub async fn get_admin_secret(
    state: State<Arc<AppState>>,
    Path(connection_id): Path<Id>,
) -> Result<Json<Secret>, IntegrationOSError> {
    let (secret_id, owner) = state
        .app_stores
        .connection
        .get_one_by_id(&connection_id.to_string())
        .await?
        .map(|c| (c.secrets_service_id, c.ownership.id))
        .ok_or(ApplicationError::not_found(
            &format!("connection with id {} not found", connection_id),
            None,
        ))?;

    Ok(Json(state.secrets_client.get(&secret_id, &owner).await?))
}
