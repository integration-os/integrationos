use super::{delete, read, PublicExt, RequestExt};
use crate::{
    config::ConnectionsConfig,
    router::ServerResponse,
    server::{AppState, AppStores},
};
use anyhow::Result;
use axum::{
    extract::State,
    routing::{delete as axum_delete, get, post},
    Extension, Json, Router,
};
use integrationos_domain::{
    access_key_data::AccessKeyData,
    access_key_prefix::AccessKeyPrefix,
    algebra::MongoStore,
    connection_definition::{ConnectionDefinitionType, Paths},
    environment::Environment,
    event_access::EventAccess,
    event_type::EventType,
    id::{prefix::IdPrefix, Id},
    ownership::Ownership,
    record_metadata::RecordMetadata,
    AccessKey, ApplicationError, IntegrationOSError, InternalError,
};
use mongodb::bson::doc;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{error, warn};
use uuid::Uuid;
use validator::Validate;

pub const DEFAULT_NAMESPACE: &str = "default";

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
impl PublicExt<EventAccess> for CreateEventAccessRequest {}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, Validate)]
#[cfg_attr(feature = "dummy", derive(fake::Dummy))]
#[serde(rename_all = "camelCase")]
pub struct CreateEventAccessPayloadWithOwnership {
    pub name: String,
    pub platform: String,
    pub namespace: Option<String>,
    pub connection_type: ConnectionDefinitionType,
    pub environment: Environment,
    pub paths: Paths,
    pub ownership: Ownership,
    pub throughput: Option<u64>,
}

impl CreateEventAccessPayloadWithOwnership {
    pub fn as_event_access(&self, config: &ConnectionsConfig) -> Result<EventAccess> {
        generate_event_access(config.clone(), self.clone())
    }
}

pub fn generate_event_access(
    config: ConnectionsConfig,
    payload: CreateEventAccessPayloadWithOwnership,
) -> Result<EventAccess> {
    let namespace = payload
        .namespace
        .unwrap_or_else(|| DEFAULT_NAMESPACE.to_string());
    let group = Uuid::new_v4().to_string().replace('-', "");

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
        throughput: payload.throughput.unwrap_or(config.event_access_throughput),
    })
}

pub async fn get_client_throughput(client_id: &str, state: &Arc<AppState>) -> Result<u64> {
    let client_record = match state
        .app_stores
        .clients
        .get_one(doc! {
            "buildableId": client_id,
        })
        .await
    {
        Ok(record) => record,
        Err(e) => {
            error!("Failed to get client throughput: {}", e);
            return Ok(state.config.event_access_throughput);
        }
    };

    Ok(client_record
        .and_then(|config| config.billing)
        .map(|billing| billing.throughput)
        .unwrap_or(state.config.event_access_throughput))
}

pub async fn create_event_access_for_new_user(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateEventAccessPayloadWithOwnership>,
) -> Result<Json<ServerResponse<EventAccess>>, IntegrationOSError> {
    if let Err(validation_errors) = req.validate() {
        warn!("Invalid payload: {:?}", validation_errors);
        return Err(ApplicationError::bad_request(
            &(format!("Invalid payload: {:?}", validation_errors)),
            None,
        ));
    }

    let throughput = get_client_throughput(&req.ownership.id, &state).await?;

    let req = CreateEventAccessPayloadWithOwnership {
        throughput: Some(throughput),
        ..req
    };

    let event_access = generate_event_access(state.config.clone(), req).map_err(|e| {
        error!("Error generating event access for new user: {:?}", e);

        InternalError::io_err("Could not generate event access", None)
    })?;

    state
        .app_stores
        .event_access
        .create_one(&event_access)
        .await
        .map_err(|e| {
            error!("Error creating event access for new user: {:?}", e);

            e
        })?;

    Ok(Json(ServerResponse::new("event_access", event_access)))
}

pub async fn create_event_access(
    Extension(access): Extension<Arc<EventAccess>>,
    State(state): State<Arc<AppState>>,
    Json(payload): Json<CreateEventAccessRequest>,
) -> Result<Json<ServerResponse<EventAccess>>, IntegrationOSError> {
    if let Err(validation_errors) = payload.validate() {
        return Err(ApplicationError::bad_request(
            &format!("Invalid payload: {:?}", validation_errors),
            None,
        ));
    }

    let throughput = get_client_throughput(&access.ownership.id, &state).await?;

    let event_access_payload = CreateEventAccessPayloadWithOwnership {
        name: payload.name.clone(),
        namespace: payload.namespace.clone(),
        platform: payload.platform.clone(),
        connection_type: payload.connection_type.clone(),
        environment: access.environment,
        paths: payload.paths.clone(),
        ownership: access.ownership.clone(),
        throughput: Some(throughput),
    };

    let event_access =
        generate_event_access(state.config.clone(), event_access_payload).map_err(|e| {
            error!("Error generating event access for existing user: {:?}", e);

            InternalError::io_err("Could not generate event access", None)
        })?;

    state
        .app_stores
        .event_access
        .create_one(&event_access)
        .await
        .map_err(|e| {
            error!("Error creating event access for existing user: {:?}", e);

            InternalError::io_err("Could not create event access", None)
        })?;

    Ok(Json(ServerResponse::new("event_access", event_access)))
}
