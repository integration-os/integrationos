use super::{delete, read, RequestExt};
use crate::{
    api_payloads::ErrorResponse,
    bad_request,
    config::Config,
    internal_server_error,
    server::{AppState, AppStores},
};
use anyhow::Result;
use axum::{
    extract::State,
    http::StatusCode,
    routing::{delete as axum_delete, get, post},
    Extension, Json, Router,
};
use integrationos_domain::{
    access_key_data::AccessKeyData,
    access_key_prefix::AccessKeyPrefix,
    algebra::{MongoStore, StoreExt},
    connection_definition::{ConnectionDefinitionType, Paths},
    environment::Environment,
    event_access::EventAccess,
    event_type::EventType,
    id::{prefix::IdPrefix, Id},
    ownership::Ownership,
    record_metadata::RecordMetadata,
    AccessKey,
};
use mongodb::bson::doc;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{error, warn};
use validator::Validate;

const DEFAULT_GROUP: &str = "event-inc-internal";
const DEFAULT_NAMESPACE: &str = "default";

pub fn get_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", post(create_event_access))
        .route("/", get(read::<CreateEventAccessRequest, EventAccess>))
        .route(
            "/:id",
            axum_delete(delete::<CreateEventAccessRequest, EventAccess>),
        )
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct CreateEventAccessRequest {
    pub name: String,
    pub group: Option<String>,
    pub platform: String,
    pub namespace: Option<String>,
    pub connection_type: ConnectionDefinitionType,
    pub paths: Paths,
}

impl RequestExt for CreateEventAccessRequest {
    type Output = EventAccess;

    fn get_store(stores: AppStores) -> MongoStore<Self::Output> {
        stores.event_access
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, Validate)]
#[cfg_attr(feature = "dummy", derive(fake::Dummy))]
#[serde(rename_all = "camelCase")]
pub struct CreateEventAccessPayloadWithOwnership {
    pub name: String,
    pub group: Option<String>,
    pub platform: String,
    pub namespace: Option<String>,
    pub connection_type: ConnectionDefinitionType,
    pub environment: Environment,
    pub paths: Paths,
    pub ownership: Ownership,
}

impl CreateEventAccessPayloadWithOwnership {
    pub fn as_event_access(&self, config: &Config) -> Result<EventAccess> {
        generate_event_access(config.clone(), self.clone())
    }
}

pub fn generate_event_access(
    config: Config,
    payload: CreateEventAccessPayloadWithOwnership,
) -> Result<EventAccess> {
    let namespace = payload
        .namespace
        .unwrap_or_else(|| DEFAULT_NAMESPACE.to_string());
    let group = payload.group.unwrap_or_else(|| DEFAULT_GROUP.to_string());

    let access_key = AccessKey {
        prefix: AccessKeyPrefix {
            environment: payload.environment,
            event_type: EventType::SecretKey,
            version: 1,
        },
        data: AccessKeyData {
            id: payload.ownership.id.to_owned().to_string(),
            namespace: namespace.clone(),
            event_type: "custom".to_owned(),
            group: group.clone(),
            event_path: payload
                .paths
                .event
                .clone()
                .unwrap_or("$.body.event".to_string()),
            event_object_id_path: Some(payload.paths.id.clone().unwrap_or("$.body.id".to_string())),
            timestamp_path: Some(
                payload
                    .paths
                    .timestamp
                    .clone()
                    .unwrap_or("$.body.id".to_string()),
            ),
            parent_access_key: None,
        },
    };

    let iv = rand::thread_rng().gen::<[u8; 16]>();
    let password = config.event_access_password.as_bytes().try_into()?;

    let encoded_access_key = access_key.encode(password, &iv)?;

    let key = format!(
        "event_access::{}::{}::{}::{}::{}",
        payload.connection_type, payload.environment, namespace, payload.platform, group
    );

    Ok(EventAccess {
        id: Id::now(IdPrefix::EventAccess),
        name: payload.name,
        namespace,
        r#type: payload.connection_type,
        group,
        platform: payload.platform,
        ownership: payload.ownership,
        key,
        paths: payload.paths,
        access_key: encoded_access_key.to_string(),
        environment: payload.environment,
        record_metadata: RecordMetadata::default(),
        throughput: config.event_access_throughput,
    })
}

pub async fn create_event_access_for_new_user(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateEventAccessPayloadWithOwnership>,
) -> Result<Json<EventAccess>, (StatusCode, Json<ErrorResponse>)> {
    if let Err(validation_errors) = req.validate() {
        warn!("Invalid payload: {:?}", validation_errors);
        return Err(bad_request!(format!(
            "Invalid payload: {:?}",
            validation_errors
        )));
    }

    let event_access = generate_event_access(state.config.clone(), req).map_err(|e| {
        error!("Error generating event access for new user: {:?}", e);

        internal_server_error!()
    })?;

    state
        .app_stores
        .event_access
        .create_one(&event_access)
        .await
        .map_err(|e| {
            error!("Error creating event access for new user: {:?}", e);

            internal_server_error!()
        })?;

    Ok(Json(event_access))
}

pub async fn create_event_access(
    Extension(user_event_access): Extension<Arc<EventAccess>>,
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateEventAccessRequest>,
) -> Result<Json<EventAccess>, (StatusCode, Json<ErrorResponse>)> {
    if let Err(validation_errors) = req.validate() {
        return Err(bad_request!(format!(
            "Invalid payload: {:?}",
            validation_errors
        )));
    }

    let event_access_payload = CreateEventAccessPayloadWithOwnership {
        name: req.name.clone(),
        group: req.group.clone(),
        namespace: req.namespace.clone(),
        platform: req.platform.clone(),
        connection_type: req.connection_type.clone(),
        environment: user_event_access.environment,
        paths: req.paths.clone(),
        ownership: user_event_access.ownership.clone(),
    };

    let event_access =
        generate_event_access(state.config.clone(), event_access_payload).map_err(|e| {
            error!("Error generating event access for existing user: {:?}", e);

            internal_server_error!()
        })?;

    state
        .app_stores
        .event_access
        .create_one(&event_access)
        .await
        .map_err(|e| {
            error!("Error creating event access for existing user: {:?}", e);

            internal_server_error!()
        })?;

    Ok(Json(event_access))
}
