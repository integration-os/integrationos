use crate::server::AppState;
use axum::{
    extract::{Path, State},
    routing::get,
    Router,
};
use integrationos_domain::{ApplicationError, Id, IntegrationOSError};
use std::sync::Arc;

pub fn get_router() -> Router<Arc<AppState>> {
    Router::new().route("/:id", get(generate_schema))
}

pub async fn generate_schema(
    state: State<Arc<AppState>>,
    Path(id): Path<Id>,
) -> Result<String, IntegrationOSError> {
    let cm_store = state.app_stores.common_model.clone();
    let ce_store = state.app_stores.common_enum.clone();

    let common_model = cm_store
        .get_one_by_id(&id.to_string())
        .await
        .map_err(|e| IntegrationOSError::from(e))?
        .ok_or(ApplicationError::not_found(
            &format!("CommonModel with id {} not found", id),
            None,
        ))?;

    let schema = common_model
        .as_typescript_schema_expanded(&cm_store, &ce_store)
        .await;

    Ok(schema)
}
