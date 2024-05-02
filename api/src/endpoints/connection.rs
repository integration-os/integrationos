use super::{delete, read, RequestExt};
use crate::{
    api_payloads::{DeleteResponse, ErrorResponse, UpdateResponse},
    bad_request,
    endpoints::event_access::{generate_event_access, CreateEventAccessPayloadWithOwnership},
    internal_server_error, not_found,
    server::{AppState, AppStores},
};
use anyhow::{bail, Result};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{delete as axum_delete, get, patch, post},
    Extension, Json, Router,
};
use chrono::Utc;
use convert_case::{Case, Casing};
use http::HeaderMap;
use integrationos_domain::{
    algebra::{MongoStore, StoreExt},
    connection_definition::ConnectionDefinition,
    event_access::EventAccess,
    id::{prefix::IdPrefix, Id},
    record_metadata::RecordMetadata,
    settings::Settings,
    Connection, Throughput,
};
use mongodb::bson::doc;
use mongodb::bson::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{collections::HashMap, sync::Arc};
use tracing::error;
use validator::Validate;

pub fn get_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", post(create_connection))
        .route("/", get(read::<CreateConnectionPayload, Connection>))
        .route("/:id", patch(update_connection))
        .route("/:id", axum_delete(delete_connection))
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct CreateConnectionPayload {
    pub connection_definition_id: Id,
    pub name: String,
    pub group: String,
    pub auth_form_data: HashMap<String, String>,
    pub active: bool,
}

async fn test_connection(
    state: &AppState,
    connection_config: &ConnectionDefinition,
    auth_form_data_value: &Value,
) -> Result<()> {
    if let Some(ref test_connection_model_config_id) = connection_config.test_connection {
        let test_connection_model_config = state
            .app_stores
            .model_config
            .get_one_by_id(&test_connection_model_config_id.to_string())
            .await?;

        let test_connection_model_config = match test_connection_model_config {
            Some(config) => Arc::new(config),
            None => {
                return Err(anyhow::anyhow!(
                    "Test connection model config {} not found",
                    test_connection_model_config_id
                ))
            }
        };

        let res = state
            .extractor_caller
            .execute_model_definition(
                &test_connection_model_config,
                HeaderMap::new(),
                &HashMap::new(),
                &Arc::new(auth_form_data_value.clone()),
                None,
            )
            .await?;

        if !res.status().is_success() {
            bail!("Test connections failed: {}", res.status());
        }
    }

    Ok(())
}

impl RequestExt for CreateConnectionPayload {
    type Output = Connection;

    fn get_store(stores: AppStores) -> MongoStore<Self::Output> {
        stores.connection
    }
}

pub async fn create_connection(
    Extension(user_event_access): Extension<Arc<EventAccess>>,
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateConnectionPayload>,
) -> Result<Json<Connection>, (StatusCode, Json<ErrorResponse>)> {
    if let Err(validation_errors) = req.validate() {
        return Err(bad_request!(format!(
            "Invalid payload: {:?}",
            validation_errors
        )));
    }

    let connection_config = match state
        .app_stores
        .connection_config
        .get_one(doc! {
            "_id": req.connection_definition_id.to_string(),
            "deleted": false
        })
        .await
    {
        Ok(Some(data)) => data,
        Ok(None) => {
            return Err(not_found!("Connection definition"));
        }
        Err(e) => {
            error!(
                "Error fetching connection definition in connection create: {:?}",
                e
            );

            return Err(internal_server_error!());
        }
    };

    let key = format!(
        "{}::{}::{}",
        user_event_access.environment,
        connection_config.platform,
        req.group.to_case(Case::Kebab)
    );

    let event_access = generate_event_access(
        state.config.clone(),
        CreateEventAccessPayloadWithOwnership {
            name: req.name.clone(),
            group: Some(req.group.clone()),
            platform: connection_config.platform.clone(),
            namespace: None,
            connection_type: connection_config.r#type.clone(),
            environment: user_event_access.environment,
            paths: connection_config.paths.clone(),
            ownership: user_event_access.ownership.clone(),
        },
    )
    .map_err(|e| {
        error!("Error creating event access for connection: {:?}", e);

        internal_server_error!()
    })?;

    let auth_form_data_value = serde_json::to_value(req.auth_form_data.clone()).map_err(|e| {
        error!("Error serializing auth form data for connection: {:?}", e);

        internal_server_error!()
    })?;

    test_connection(&state, &connection_config, &auth_form_data_value)
        .await
        .map_err(|e| {
            error!(
            "Error executing model definition in connections create for connection testing: {:?}",
            e
        );

            bad_request!("Invalid connection credentials")
        })?;

    let secret_result = state
        .secrets_client
        .encrypt(
            user_event_access.ownership.id.to_string(),
            &auth_form_data_value,
        )
        .await
        .map_err(|e| {
            error!("Error creating secret for connection: {:?}", e);

            internal_server_error!()
        })?;

    let connection = Connection {
        id: Id::new(IdPrefix::Connection, Utc::now()),
        platform_version: connection_config.clone().platform_version,
        connection_definition_id: req.connection_definition_id,
        r#type: connection_config.to_connection_type(),
        name: req.name,
        key: key.clone().into(),
        group: req.group,
        platform: connection_config.platform.into(),
        environment: event_access.environment,
        secrets_service_id: secret_result.id,
        event_access_id: event_access.id,
        access_key: event_access.access_key,
        settings: connection_config.settings,
        throughput: Throughput { key, limit: 100 },
        ownership: event_access.ownership,
        oauth: None,
        record_metadata: RecordMetadata::default(),
    };

    state
        .app_stores
        .connection
        .create_one(&connection)
        .await
        .map_err(|e| {
            error!("Error creating connection: {:?}", e);

            internal_server_error!()
        })?;

    Ok(Json(connection))
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct UpdateConnectionPayload {
    pub name: Option<String>,
    pub group: Option<String>,
    pub settings: Option<Settings>,
    pub throughput: Option<Throughput>,
    pub auth_form_data: Option<HashMap<String, String>>,
    pub active: Option<bool>,
}

pub async fn update_connection(
    Extension(event_access): Extension<Arc<EventAccess>>,
    Path(id): Path<String>,
    State(state): State<Arc<AppState>>,
    Json(req): Json<UpdateConnectionPayload>,
) -> Result<Json<UpdateResponse>, (StatusCode, Json<ErrorResponse>)> {
    let Some(mut connection) = (match state.app_stores.connection.get_one_by_id(&id).await {
        Ok(connection) => connection,
        Err(e) => {
            error!("Error fetching connection for update: {:?}", e);

            return Err(internal_server_error!());
        }
    }) else {
        return Err(not_found!("Connection"));
    };

    if connection.ownership != event_access.ownership
        || connection.environment != event_access.environment
    {
        return Err(not_found!("Connection"));
    }

    if let Some(name) = req.name {
        connection.name = name;
    }

    if let Some(group) = req.group {
        connection.group = group.clone();
        connection.key = format!("{}::{}", connection.platform, group).into();
    }

    if let Some(settings) = req.settings {
        connection.settings = settings;
    }

    if let Some(throughput) = req.throughput {
        connection.throughput = throughput;
    }

    if let Some(auth_form_data) = req.auth_form_data {
        let auth_form_data_value = serde_json::to_value(auth_form_data).map_err(|e| {
            error!(
                "Error serializing auth form data for connection update: {:?}",
                e
            );

            internal_server_error!()
        })?;

        let connection_config = match state
            .app_stores
            .connection_config
            .get_one(doc! {
                "_id": connection.connection_definition_id.to_string(),
                "deleted": false
            })
            .await
        {
            Ok(Some(data)) => data,
            Ok(None) => {
                return Err(not_found!("Connection definition"));
            }
            Err(e) => {
                error!(
                    "Error fetching connection definition in connection update: {:?}",
                    e
                );

                return Err(internal_server_error!());
            }
        };

        test_connection(&state, &connection_config, &auth_form_data_value)
            .await
            .map_err(|e| {
                error!("Error executing model definition in connections update for connection testing: {:?}", e);

                bad_request!("Invalid connection credentials")
            })?;

        let secret_result = state
            .secrets_client
            .encrypt(event_access.ownership.id.to_string(), &auth_form_data_value)
            .await
            .map_err(|e| {
                error!("Error creating secret for connection update: {:?}", e);

                internal_server_error!()
            })?;

        connection.secrets_service_id = secret_result.id;
    }

    if let Some(active) = req.active {
        connection.record_metadata.active = active;
    }

    let Ok(document) = bson::to_document(&connection) else {
        error!("Could not serialize connection into document");

        return Err(internal_server_error!());
    };

    connection
        .record_metadata
        .mark_updated(&event_access.ownership.id);

    match state
        .app_stores
        .connection
        .update_one(
            &id,
            doc! {
                "$set": document
            },
        )
        .await
    {
        Ok(_) => Ok(Json(UpdateResponse { id })),
        Err(e) => {
            error!("Error updating connection: {:?}", e);

            Err(internal_server_error!())
        }
    }
}

pub async fn delete_connection(
    Extension(user_event_access): Extension<Arc<EventAccess>>,
    Path(id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<DeleteResponse>, (StatusCode, Json<ErrorResponse>)> {
    let Json(found_connection) = delete::<CreateConnectionPayload, Connection>(
        Some(Extension(user_event_access.clone())),
        Path(id.clone()),
        State(state.clone()),
    )
    .await?;

    let partial_cursor_key = format!(
        "{}::{}::{}",
        user_event_access.ownership.id, id, found_connection.key
    );

    let mongo_regex = Regex {
        pattern: format!("^{}::", partial_cursor_key.replace('.', "\\.")),
        options: "i".to_string(),
    };

    // Delete cursors by adding "deleted/" to the key
    state
        .app_stores
        .cursors
        .update_many_with_aggregation_pipeline(
            doc! {
                "key": mongo_regex
            },
            &Vec::from([doc! {
                "$set": {
                    "key": {
                        "$concat": ["deleted/", "$key"]
                    }
                }
            }]),
        )
        .await
        .map_err(|e| {
            error!("Error deleting cursors for connection {:?}: {:?}", id, e);

            internal_server_error!()
        })?;

    Ok(Json(DeleteResponse { id }))
}
