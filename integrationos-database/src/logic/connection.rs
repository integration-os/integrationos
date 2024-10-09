use crate::server::AppState;
use axum::{
    extract::{Query, State},
    routing::{get, post},
    Json, Router,
};
use integrationos_domain::IntegrationOSError;
use serde::Deserialize;
use serde_json::Value;
use std::{collections::HashMap, sync::Arc};

pub fn get_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", post(get_raw))
        .route("/probe", get(test_probe))
}

async fn test_probe(
    state: State<Arc<AppState>>,
) -> Result<Json<Vec<HashMap<String, Value>>>, IntegrationOSError> {
    state.storage.execute_raw("SELECT 1").await.map(Json)
}

#[derive(Deserialize, Debug)]
struct RawQuery {
    query: String,
}

async fn get_raw(
    state: State<Arc<AppState>>,
    query: Query<RawQuery>,
) -> Result<Json<Vec<HashMap<String, Value>>>, IntegrationOSError> {
    state.storage.execute_raw(&query.query).await.map(Json)
}
