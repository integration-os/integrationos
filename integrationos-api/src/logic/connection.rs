use super::{delete, read, PublicExt, RequestExt};
use crate::{
    logic::event_access::{
        generate_event_access, get_client_throughput, CreateEventAccessPayloadWithOwnership,
    },
    router::ServerResponse,
    server::{AppState, AppStores},
};
use anyhow::{bail, Result};
use axum::{
    extract::{Path, State},
    routing::{delete as axum_delete, get, patch, post},
    Extension, Json, Router,
};
use chrono::Utc;
use http::HeaderMap;
use integrationos_domain::{
    algebra::MongoStore,
    connection_definition::ConnectionDefinition,
    domain::connection::SanitizedConnection,
    event_access::EventAccess,
    id::{prefix::IdPrefix, Id},
    record_metadata::RecordMetadata,
    settings::Settings,
    ApplicationError, Connection, IntegrationOSError, InternalError, Throughput,
};
use mongodb::bson::doc;
use mongodb::bson::Regex;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
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

impl PublicExt<Connection> for CreateConnectionPayload {
    fn public(input: Connection) -> Value {
        SanitizedConnection {
            id: input.id,
            platform_version: input.platform_version,
            connection_definition_id: input.connection_definition_id,
            r#type: input.r#type,
            name: input.name,
            key: input.key,
            group: input.group,
            environment: input.environment,
            platform: input.platform,
            secrets_service_id: input.secrets_service_id,
            event_access_id: input.event_access_id,
            settings: input.settings,
            throughput: input.throughput,
            ownership: input.ownership,
            oauth: input.oauth,
            record_metadata: input.record_metadata,
        }
        .to_value()
    }
}
impl RequestExt for CreateConnectionPayload {
    type Output = Connection;

    fn get_store(stores: AppStores) -> MongoStore<Self::Output> {
        stores.connection
    }
}

pub async fn create_connection(
    Extension(access): Extension<Arc<EventAccess>>,
    State(state): State<Arc<AppState>>,
    Json(payload): Json<CreateConnectionPayload>,
) -> Result<Json<SanitizedConnection>, IntegrationOSError> {
    if let Err(validation_errors) = payload.validate() {
        return Err(ApplicationError::not_found(
            &format!("Invalid payload: {:?}", validation_errors),
            None,
        ));
    }

    let connection_config = match state
        .app_stores
        .connection_config
        .get_one(doc! {
            "_id": payload.connection_definition_id.to_string(),
            "deleted": false
        })
        .await
    {
        Ok(Some(data)) => data,
        Ok(None) => {
            return Err(ApplicationError::not_found(
                &format!(
                    "Connection definition with id {} not found",
                    payload.connection_definition_id
                ),
                None,
            ));
        }
        Err(e) => {
            error!(
                "Error fetching connection definition in connection create: {:?}",
                e
            );

            return Err(e);
        }
    };

    let key = format!(
        "{}::{}::{}",
        access.environment,
        connection_config.platform,
        payload.group.replace([':', ' '], "_")
    );

    let throughput = get_client_throughput(&access.ownership.id, &state).await?;

    let event_access = generate_event_access(
        state.config.clone(),
        CreateEventAccessPayloadWithOwnership {
            name: payload.name.clone(),
            group: Some(payload.group.clone()),
            platform: connection_config.platform.clone(),
            namespace: None,
            connection_type: connection_config.r#type.clone(),
            environment: access.environment,
            paths: connection_config.paths.clone(),
            ownership: access.ownership.clone(),
            throughput: Some(throughput),
        },
    )
    .map_err(|e| {
        error!("Error creating event access for connection: {:?}", e);

        e
    })?;

    let auth_form_data_value =
        serde_json::to_value(payload.auth_form_data.clone()).map_err(|e| {
            error!("Error serializing auth form data for connection: {:?}", e);

            ApplicationError::bad_request(&format!("Invalid auth form data: {:?}", e), None)
        })?;

    test_connection(&state, &connection_config, &auth_form_data_value)
        .await
        .map_err(|e| {
            error!(
            "Error executing model definition in connections create for connection testing: {:?}",
            e
        );

            ApplicationError::bad_request("Invalid connection credentials: {:?}", None)
        })?;

    let secret_result = state
        .secrets_client
        .encrypt(access.ownership.id.to_string(), &auth_form_data_value)
        .await
        .map_err(|e| {
            error!("Error creating secret for connection: {:?}", e);

            e
        })?;

    let connection = Connection {
        id: Id::new(IdPrefix::Connection, Utc::now()),
        platform_version: connection_config.clone().platform_version,
        connection_definition_id: payload.connection_definition_id,
        r#type: connection_config.to_connection_type(),
        name: payload.name,
        key: key.clone().into(),
        group: payload.group,
        platform: connection_config.platform.into(),
        environment: event_access.environment,
        secrets_service_id: secret_result.id,
        event_access_id: event_access.id,
        access_key: event_access.access_key,
        settings: connection_config.settings,
        throughput: Throughput {
            key,
            limit: throughput,
        },
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

            e
        })?;

    Ok(Json(SanitizedConnection {
        id: connection.id,
        platform_version: connection.platform_version,
        connection_definition_id: connection.connection_definition_id,
        r#type: connection.r#type,
        name: connection.name,
        key: connection.key,
        group: connection.group,
        environment: connection.environment,
        platform: connection.platform,
        secrets_service_id: connection.secrets_service_id,
        event_access_id: connection.event_access_id,
        settings: connection.settings,
        throughput: connection.throughput,
        ownership: connection.ownership,
        oauth: connection.oauth,
        record_metadata: connection.record_metadata,
    }))
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
) -> Result<Json<ServerResponse<Value>>, IntegrationOSError> {
    let Some(mut connection) = (match state.app_stores.connection.get_one_by_id(&id).await {
        Ok(connection) => connection,
        Err(e) => {
            error!("Error fetching connection for update: {:?}", e);

            return Err(e);
        }
    }) else {
        return Err(ApplicationError::not_found(
            &format!("Connection with id {id} not found"),
            None,
        ));
    };

    if connection.ownership != event_access.ownership
        || connection.environment != event_access.environment
    {
        return Err(ApplicationError::forbidden(
            "You do not have permission to update this connection",
            None,
        ));
    }

    if let Some(name) = req.name {
        connection.name = name;
    }

    if let Some(group) = req.group {
        connection.group.clone_from(&group);
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

            ApplicationError::bad_request(&format!("Invalid auth form data: {:?}", e), None)
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
                return Err(ApplicationError::not_found(
                    "Connection definition not found",
                    None,
                ));
            }
            Err(e) => {
                error!(
                    "Error fetching connection definition in connection update: {:?}",
                    e
                );

                return Err(e);
            }
        };

        test_connection(&state, &connection_config, &auth_form_data_value)
            .await
            .map_err(|e| {
                error!("Error executing model definition in connections update for connection testing: {:?}", e);

                ApplicationError::bad_request(&format!("Invalid auth form data: {:?}", e), None)
            })?;

        let secret_result = state
            .secrets_client
            .encrypt(event_access.ownership.id.to_string(), &auth_form_data_value)
            .await
            .map_err(|e| {
                error!("Error creating secret for connection update: {:?}", e);

                e
            })?;

        connection.secrets_service_id = secret_result.id;
    }

    if let Some(active) = req.active {
        connection.record_metadata.active = active;
    }

    let Ok(document) = bson::to_document(&connection) else {
        error!("Could not serialize connection into document");

        return Err(InternalError::serialize_error(
            "Could not serialize connection into document",
            None,
        ));
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
        Ok(_) => Ok(Json(ServerResponse::new(
            "connection",
            json!({
                id: connection.id,
            }),
        ))),
        Err(e) => {
            error!("Error updating connection: {:?}", e);

            Err(e)
        }
    }
}

pub async fn delete_connection(
    Extension(access): Extension<Arc<EventAccess>>,
    Path(id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<ServerResponse<Value>>, IntegrationOSError> {
    let connection = delete::<CreateConnectionPayload, Connection>(
        Some(Extension(access.clone())),
        Path(id.clone()),
        State(state.clone()),
    )
    .await?;

    let partial_cursor_key = format!("{}::{}::{}", access.ownership.id, id, connection.args.key);

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

            e
        })?;

    Ok(Json(ServerResponse::new(
        "connection",
        json!({
            id: connection.args.id,
        }),
    )))
}
