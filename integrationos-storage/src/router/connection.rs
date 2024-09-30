use crate::server::AppState;
use axum::{extract::State, Json};
use integrationos_domain::IntegrationOSError;
use std::{collections::HashMap, sync::Arc};

pub async fn get_raw(
    state: State<Arc<AppState>>,
) -> Result<Json<Vec<HashMap<String, Option<String>>>>, IntegrationOSError> {
    state.storage.execute_raw("SELECT 1").await.map(Json)
}
