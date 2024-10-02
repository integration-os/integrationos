use super::{get_connection, INTEGRATION_OS_PASSTHROUGH_HEADER};
use crate::{domain::config::Headers, domain::metrics::Metric, server::AppState};
use axum::{
    extract::{Path, Query, State},
    response::{IntoResponse, Response},
    routing::{delete, get, patch, post, put},
    Extension, Json, Router,
};
use bson::doc;
use convert_case::{Case, Casing};
use http::{HeaderMap, HeaderName};
use integrationos_domain::{
    connection_model_definition::CrudAction, destination::Action,
    encrypted_access_key::EncryptedAccessKey, encrypted_data::PASSWORD_LENGTH,
    event_access::EventAccess, AccessKey, ApplicationError, Event, IntegrationOSError,
    InternalError,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{collections::HashMap, sync::Arc};
use tracing::error;

pub fn get_router() -> Router<Arc<AppState>> {
    Router::new().route("/:model/:id", get(raw_request))
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct StorageParams {
    pub table: String,
}

pub async fn raw_request(
    state: State<Arc<AppState>>,
    Path(params): Path<StorageParams>,
) -> Result<Json<Value>, IntegrationOSError> {
    todo!()
    // let storage = state.storage.clone();
    // let table = params.table;
    // let res = storage.execute_raw(&format!("SELECT * FROM {}", table)).await;
    // match res {
    //     Ok(rows) => {
    //         let json_results = process_rows(rows)?;
    //         Ok(Json(ServerResponse::new(
    //             "storage",
    //             json_results,
    //         )))
    //     }
    //     Err(e) => {
    //         error!("Error reading from storage: {e}");
    //         Err(e)
    //     }
    // }
}
