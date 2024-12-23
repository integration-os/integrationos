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
    event_access::EventAccess, AccessKey, ApplicationError, Event, InternalError,
};
use integrationos_unified::domain::{RequestCrud, RequestCrudBuilder};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{collections::HashMap, sync::Arc};
use tracing::error;

pub fn get_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/:model/:id", get(get_request))
        .route("/:model/:id", patch(update_request))
        .route("/:model", put(upsert_request))
        .route("/:model", get(list_request))
        .route("/:model/count", get(count_request))
        .route("/:model", post(create_request))
        .route("/:model/:id", delete(delete_request))
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PathParams {
    pub id: String,
    pub model: String,
}

pub async fn get_request(
    access: Extension<Arc<EventAccess>>,
    Extension(passthrough): Extension<Arc<bool>>,
    state: State<Arc<AppState>>,
    Path(params): Path<PathParams>,
    headers: HeaderMap,
    query_params: Option<Query<HashMap<String, String>>>,
) -> impl IntoResponse {
    process_request(
        access,
        state,
        headers,
        query_params,
        Action::Unified {
            name: params.model.to_case(Case::Pascal).into(),
            action: CrudAction::GetOne,
            id: Some(params.id.into()),
            passthrough: *passthrough,
        },
        None,
    )
    .await
}

const META: &str = "meta";

pub async fn update_request(
    access: Extension<Arc<EventAccess>>,
    Extension(passthrough): Extension<Arc<bool>>,
    state: State<Arc<AppState>>,
    Path(params): Path<PathParams>,
    headers: HeaderMap,
    query_params: Option<Query<HashMap<String, String>>>,
    Json(body): Json<Value>,
) -> impl IntoResponse {
    process_request(
        access,
        state,
        headers,
        query_params,
        Action::Unified {
            name: params.model.to_case(Case::Pascal).into(),
            action: CrudAction::Update,
            id: Some(params.id.into()),
            passthrough: *passthrough,
        },
        Some(body),
    )
    .await
}

pub async fn upsert_request(
    access: Extension<Arc<EventAccess>>,
    Extension(passthrough): Extension<Arc<bool>>,
    state: State<Arc<AppState>>,
    Path(model): Path<String>,
    headers: HeaderMap,
    query_params: Option<Query<HashMap<String, String>>>,
    Json(body): Json<Value>,
) -> impl IntoResponse {
    process_request(
        access,
        state,
        headers,
        query_params,
        Action::Unified {
            name: model.to_case(Case::Pascal).into(),
            action: CrudAction::Upsert,
            id: None,
            passthrough: *passthrough,
        },
        Some(body),
    )
    .await
}

pub async fn list_request(
    access: Extension<Arc<EventAccess>>,
    Extension(passthrough): Extension<Arc<bool>>,
    state: State<Arc<AppState>>,
    Path(model): Path<String>,
    headers: HeaderMap,
    query_params: Option<Query<HashMap<String, String>>>,
) -> impl IntoResponse {
    process_request(
        access,
        state,
        headers,
        query_params,
        Action::Unified {
            name: model.to_case(Case::Pascal).into(),
            action: CrudAction::GetMany,
            id: None,
            passthrough: *passthrough,
        },
        None,
    )
    .await
}

pub async fn count_request(
    access: Extension<Arc<EventAccess>>,
    Extension(passthrough): Extension<Arc<bool>>,
    state: State<Arc<AppState>>,
    Path(model): Path<String>,
    headers: HeaderMap,
    query_params: Option<Query<HashMap<String, String>>>,
) -> impl IntoResponse {
    process_request(
        access,
        state,
        headers,
        query_params,
        Action::Unified {
            name: model.to_case(Case::Pascal).into(),
            action: CrudAction::GetCount,
            id: None,
            passthrough: *passthrough,
        },
        None,
    )
    .await
}

pub async fn create_request(
    access: Extension<Arc<EventAccess>>,
    state: State<Arc<AppState>>,
    Extension(passthrough): Extension<Arc<bool>>,
    Path(model): Path<String>,
    headers: HeaderMap,
    query_params: Option<Query<HashMap<String, String>>>,
    Json(body): Json<Value>,
) -> impl IntoResponse {
    process_request(
        access,
        state,
        headers,
        query_params,
        Action::Unified {
            name: model.to_case(Case::Pascal).into(),
            action: CrudAction::Create,
            id: None,
            passthrough: *passthrough,
        },
        Some(body),
    )
    .await
}

pub async fn delete_request(
    access: Extension<Arc<EventAccess>>,
    Extension(passthrough): Extension<Arc<bool>>,
    state: State<Arc<AppState>>,
    Path(params): Path<PathParams>,
    headers: HeaderMap,
    query_params: Option<Query<HashMap<String, String>>>,
) -> impl IntoResponse {
    process_request(
        access,
        state,
        headers,
        query_params,
        Action::Unified {
            name: params.model.to_case(Case::Pascal).into(),
            action: CrudAction::Delete,
            id: Some(params.id.into()),
            passthrough: *passthrough,
        },
        None,
    )
    .await
}

pub async fn process_request(
    Extension(access): Extension<Arc<EventAccess>>,
    State(state): State<Arc<AppState>>,
    mut headers: HeaderMap,
    query_params: Option<Query<HashMap<String, String>>>,
    action: Action,
    payload: Option<Value>,
) -> impl IntoResponse {
    let Some(connection_key_header) = headers.get(&state.config.headers.connection_header) else {
        return Err(ApplicationError::bad_request(
            "Missing connection key header",
            None,
        ));
    };
    let connection = get_connection(
        access.as_ref(),
        connection_key_header,
        &state.app_stores,
        &state.connections_cache,
    )
    .await
    .map_err(|e| {
        error!("Error getting connection: {:?}", e);
        e
    })?;

    let Query(query_params) = query_params.unwrap_or_default();

    let access_key_header_value = headers.get(&state.config.headers.auth_header).cloned();

    remove_event_headers(&mut headers, &state.config.headers);

    let Action::Unified {
        name: model_name,
        action: action_name,
        ..
    } = &action
    else {
        return Err(ApplicationError::bad_request("Invalid action", None));
    };
    let event_name = format!(
        "{}::{}::{}::{}",
        connection.platform, connection.platform_version, model_name, action_name,
    );

    let mut response = state
        .extractor_caller
        .dispatch_unified_request(
            connection.clone(),
            action.clone(),
            state.config.environment,
            RequestCrudBuilder::default()
                .headers(headers)
                .query_params(query_params)
                .body(payload)
                .build()
                .map_err(|e| {
                    error!("Error building request crud: {e}");
                    InternalError::invalid_argument(
                        &format!("Error building request crud: {e}"),
                        None,
                    )
                })?,
        )
        .await
        .inspect_err(|e| {
            error!(
                "Error executing connection model definition in unified endpoint: {}",
                e.to_string()
            );
        })?;

    *response.response.headers_mut() = response
        .response
        .headers()
        .iter()
        .map(|(key, value)| {
            (
                HeaderName::try_from(format!("{INTEGRATION_OS_PASSTHROUGH_HEADER}-{key}")).unwrap(),
                value.clone(),
            )
        })
        .collect::<HeaderMap>();

    let (parts, body) = response.response.into_parts();
    let mut metadata = body.get(META).unwrap_or(&response.metadata.as_value()).clone();

    if let Some(Ok(encrypted_access_key)) =
        access_key_header_value.map(|v| v.to_str().map(|s| s.to_string()))
    {
        if let Ok(encrypted_access_key) = EncryptedAccessKey::parse(&encrypted_access_key) {
            let password: [u8; PASSWORD_LENGTH] = state
                .config
                .event_access_password
                .as_bytes()
                .try_into()
                .map_err(|e| {
                    error!("event_access_password is not 32 bytes in length: {e}");
                    InternalError::decryption_error(
                        "event_access_password is not 32 bytes in length",
                        None,
                    )
                    .set_meta(&metadata)
                })?;

            let access_key = AccessKey::parse(&encrypted_access_key, &password).map_err(|e| {
                error!("Could not decrypt access key: {e}");
                InternalError::decryption_error("Could not decrypt access key", None)
                    .set_meta(&metadata)
            })?;
            let status_code = parts.status.as_u16();

            if let Some(meta) = metadata.as_object_mut() {
                meta.insert("status_code".to_string(), json!(status_code));
                meta.insert("path".to_string(), json!("v1/unified"));
            };

            let body = serde_json::to_string(&json!({
                META: metadata,
            }))
            .map_err(|e| {
                error!("Could not serialize meta body to string: {e}");
                InternalError::invalid_argument("Could not serialize meta body to string", None)
                    .set_meta(&metadata)
            })?;

            let name = if parts.status.is_success() {
                format!("{event_name}::request-succeeded",)
            } else {
                format!("{event_name}::request-failed",)
            };
            let event = Event::new(
                &access_key,
                &encrypted_access_key,
                &name,
                parts.headers.clone(),
                body,
            );
            if let Err(e) = state.event_tx.send(event).await {
                error!("Could not send event to receiver: {e}");
            }
        }
    };

    let metric = Metric::unified(connection.clone(), action);
    if let Err(e) = state.metric_tx.send(metric).await {
        error!("Could not send metric to receiver: {e}");
    }

    let response = Response::from_parts(parts, ());

    if response.status().is_client_error() || response.status().is_server_error() {
        let body = json!({
            META: metadata,
            "error": body,
        });

        Ok((response, Json(body)))
    } else {
        Ok((response, Json(body)))
    }
}

fn remove_event_headers(headers: &mut HeaderMap, headers_config: &Headers) {
    headers.remove(&headers_config.auth_header);
    headers.remove(&headers_config.connection_header);
    headers.remove(&headers_config.enable_passthrough_header);
}
