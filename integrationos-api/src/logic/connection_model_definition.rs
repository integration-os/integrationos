use super::{create, delete, read, update, HookExt, PublicExt, RequestExt};
use crate::{
    router::ServerResponse,
    server::{AppState, AppStores},
};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{patch, post},
    Extension, Json, Router,
};
use chrono::Utc;
use http::HeaderMap;
use integrationos_domain::{
    algebra::MongoStore,
    api_model_config::{
        ApiModelConfig, AuthMethod, ModelPaths, ResponseBody, SamplesInput, SchemasInput,
    },
    connection_model_definition::{
        ConnectionModelDefinition, CrudAction, CrudMapping, ExtractorConfig, PlatformInfo,
        TestConnection, TestConnectionState,
    },
    event_access::EventAccess,
    id::{prefix::IdPrefix, Id},
    ApplicationError, IntegrationOSError, InternalError,
};
use mongodb::bson::doc;
use semver::Version;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::{BTreeMap, HashMap},
    sync::Arc,
};
use tracing::error;

pub fn get_router() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/",
            post(create::<CreateRequest, ConnectionModelDefinition>)
                .get(read::<CreateRequest, ConnectionModelDefinition>),
        )
        .route(
            "/:id",
            patch(update::<CreateRequest, ConnectionModelDefinition>)
                .delete(delete::<CreateRequest, ConnectionModelDefinition>),
        )
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TestConnectionPayload {
    pub connection_key: String,
    pub request: TestConnectionRequest,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[cfg_attr(feature = "dummy", derive(fake::Dummy))]
#[serde(rename_all = "camelCase")]
pub struct TestConnectionRequest {
    #[serde(
        with = "http_serde_ext::header_map::option",
        skip_serializing_if = "Option::is_none",
        default
    )]
    pub headers: Option<HeaderMap>,
    pub query_params: Option<HashMap<String, String>>,
    pub path_params: Option<HashMap<String, String>>,
    pub body: Option<Value>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TestConnectionResponse {
    #[serde(with = "http_serde_ext::status_code")]
    pub code: StatusCode,
    pub status: TestConnection,
    pub meta: Meta,
    pub response: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Meta {
    pub timestamp: u64,
    pub platform: String,
    pub platform_version: String,
    pub connection_definition_id: Id,
    pub connection_key: String,
    pub model_name: String,
    #[serde(with = "http_serde_ext::method")]
    pub action: http::Method,
}

pub async fn test_connection_model_definition(
    Extension(access): Extension<Arc<EventAccess>>,
    Path(id): Path<String>,
    State(state): State<Arc<AppState>>,
    Json(payload): Json<TestConnectionPayload>,
) -> Result<Json<ServerResponse<TestConnectionResponse>>, IntegrationOSError> {
    let connection = match state
        .app_stores
        .connection
        .get_one(doc! {
            "key": &payload.connection_key,
            "ownership.buildableId": access.ownership.id.as_ref(),
            "deleted": false
        })
        .await
    {
        Ok(Some(data)) => data,
        Ok(None) => {
            return Err(ApplicationError::not_found(
                &format!("Connection with key {} not found", payload.connection_key),
                None,
            ));
        }
        Err(e) => {
            error!("Error fetching connection in testing endpoint: {:?}", e);

            return Err(e);
        }
    };

    let connection_model_definition = match state
        .app_stores
        .model_config
        .get_one(doc! {
            "_id": id,
            "active": false, // Cannot test an active model definition
            "deleted": false
        })
        .await
    {
        Ok(Some(data)) => data,
        Ok(None) => {
            return Err(ApplicationError::not_found(
                "Inactive Connection Model Definition Record",
                None,
            ));
        }
        Err(e) => {
            error!(
                "Error fetching inactive connection model definition in testing endpoint: {:?}",
                e
            );

            return Err(e);
        }
    };

    let secret_result = state
        .secrets_client
        .get(&connection.secrets_service_id, &connection.ownership.id)
        .await
        .map_err(|e| {
            error!("Error decripting secret for connection: {:?}", e);

            e
        })?;

    let mut secret_result = secret_result.as_value()?;

    let request_string: String = serde_json::to_string(&payload.request.clone()).map_err(|e| {
        error!(
            "Error converting request to json string in testing endpoint: {:?}",
            e
        );

        InternalError::script_error("Could not serialize request payload", None)
    })?;

    // Add path params to template context
    if let Some(path_params) = payload.request.path_params {
        for (key, val) in path_params {
            secret_result[key] = Value::String(val);
        }
    }

    let request_body_vec = payload
        .request
        .body
        .map(|body| body.to_string().into_bytes());
    let model_execution_result = state
        .extractor_caller
        .execute_model_definition(
            &Arc::new(connection_model_definition.clone()),
            payload.request.headers.unwrap_or_default(),
            &payload.request.query_params.unwrap_or(HashMap::new()),
            &Arc::new(secret_result),
            request_body_vec,
        )
        .await
        .map_err(|e| {
            error!("Error executing connection model definition: {:?}", e);

            e
        })?;

    let status_code = model_execution_result.status();

    let response_body = model_execution_result.text().await.map_err(|e| {
        error!("Could not get text from test connection failure: {e}");

        InternalError::unknown("Could not get text from test connection", None)
    })?;

    let status = match status_code {
        status if status.is_success() => TestConnection {
            last_tested_at: Utc::now().timestamp_millis(),
            state: TestConnectionState::Success {
                response: response_body.clone(),
                request_payload: request_string,
            },
        },
        _ => TestConnection {
            last_tested_at: Utc::now().timestamp_millis(),
            state: TestConnectionState::Failure {
                message: response_body.clone(),
                request_payload: request_string,
            },
        },
    };

    let status_bson = bson::to_bson_with_options(&status, Default::default()).map_err(|e| {
        error!("Error serializing status to BSON: {:?}", e);

        InternalError::serialize_error("Could not serialize status to BSON", None)
    })?;

    state
        .app_stores
        .model_config
        .update_one(
            &connection_model_definition.id.to_string(),
            doc! {
                "$set": {
                    "testConnectionStatus": status_bson
                }
            },
        )
        .await
        .map_err(|e| {
            error!(
                "Error updating connection model definition in testing endpoint: {:?}",
                e
            );

            e
        })?;

    let response = TestConnectionResponse {
        code: status_code,
        status,
        response: response_body,
        meta: Meta {
            timestamp: Utc::now().timestamp_millis() as u64,
            platform: connection.platform.to_string(),
            platform_version: connection.platform_version.clone(),
            connection_definition_id: connection_model_definition.connection_definition_id,
            connection_key: connection.key.to_string(),
            model_name: connection_model_definition.model_name.clone(),
            action: connection_model_definition.action.clone(),
        },
    };

    Ok(Json(ServerResponse::new(
        "connection_model_definition",
        response,
    )))
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[cfg_attr(feature = "dummy", derive(fake::Dummy))]
#[serde(rename_all = "camelCase")]
pub struct CreateRequest {
    #[serde(rename = "_id")]
    pub id: Option<Id>,
    pub connection_platform: String,
    pub connection_definition_id: Id,
    pub platform_version: String,
    pub title: String,
    pub name: String,
    pub model_name: String,
    pub base_url: String,
    pub path: String,
    pub auth_method: AuthMethod,
    pub action_name: CrudAction,
    #[serde(with = "http_serde_ext::method", rename = "action")]
    pub http_method: http::Method,
    #[serde(
        with = "http_serde_ext::header_map::option",
        skip_serializing_if = "Option::is_none",
        default
    )]
    pub headers: Option<HeaderMap>,
    pub query_params: Option<BTreeMap<String, String>>,
    #[serde(flatten, skip_serializing_if = "Option::is_none")]
    pub extractor_config: Option<ExtractorConfig>,
    pub schemas: SchemasInput,
    pub samples: SamplesInput,
    pub responses: Vec<ResponseBody>,
    pub version: Version, // the event-inc-version
    pub is_default_crud_mapping: Option<bool>,
    pub mapping: Option<CrudMapping>,
    pub paths: Option<ModelPaths>,
    pub supported: Option<bool>,
    pub active: Option<bool>,
}

impl HookExt<ConnectionModelDefinition> for CreateRequest {}
impl PublicExt<ConnectionModelDefinition> for CreateRequest {}

impl RequestExt for CreateRequest {
    type Output = ConnectionModelDefinition;

    fn from(&self) -> Option<Self::Output> {
        let key = format!(
            "api::{}::{}::{}::{}::{}::{}",
            self.connection_platform,
            self.platform_version,
            self.model_name,
            self.action_name,
            self.path,
            self.name
        )
        .to_lowercase();

        let mut record = Self::Output {
            id: self
                .id
                .unwrap_or_else(|| Id::now(IdPrefix::ConnectionModelDefinition)),
            connection_platform: self.connection_platform.clone(),
            connection_definition_id: self.connection_definition_id,
            platform_version: self.platform_version.clone(),
            key,
            title: self.title.clone(),
            name: self.name.clone(),
            model_name: self.model_name.clone(),
            platform_info: PlatformInfo::Api(ApiModelConfig {
                base_url: self.base_url.clone(),
                path: self.path.clone(),
                content: Default::default(),
                auth_method: self.auth_method.clone(),
                headers: self.headers.clone(),
                query_params: self.query_params.clone(),
                schemas: self.schemas.clone(),
                samples: self.samples.clone(),
                responses: self.responses.clone(),
                paths: self.paths.clone(),
            }),
            action: self.http_method.clone(),
            action_name: self.action_name.clone(),
            extractor_config: self.extractor_config.clone(),
            test_connection_status: TestConnection::default(),
            is_default_crud_mapping: self.is_default_crud_mapping,
            mapping: self.mapping.clone(),
            record_metadata: Default::default(),
            supported: self.supported.unwrap_or(false),
        };
        record.record_metadata.version = self.version.clone();
        Some(record)
    }

    fn update(&self, mut record: Self::Output) -> Self::Output {
        let key = format!(
            "api::{}::{}::{}::{}::{}::{}",
            self.connection_platform,
            self.platform_version,
            self.model_name,
            self.action_name,
            self.path,
            self.name
        )
        .to_lowercase();

        record.key = key;
        record
            .connection_platform
            .clone_from(&self.connection_platform);
        record.connection_definition_id = self.connection_definition_id;
        record.platform_version.clone_from(&self.platform_version);
        record.title.clone_from(&self.title);
        record.name.clone_from(&self.name);
        record.action = self.http_method.clone();
        record.action_name = self.action_name.clone();
        record.platform_info = PlatformInfo::Api(ApiModelConfig {
            base_url: self.base_url.clone(),
            path: self.path.clone(),
            content: Default::default(),
            auth_method: self.auth_method.clone(),
            headers: self.headers.clone(),
            query_params: self.query_params.clone(),
            schemas: self.schemas.clone(),
            samples: self.samples.clone(),
            responses: self.responses.clone(),
            paths: self.paths.clone(),
        });
        record.mapping.clone_from(&self.mapping);
        record.extractor_config.clone_from(&self.extractor_config);
        record.record_metadata.version = self.version.clone();

        if let Some(supported) = self.supported {
            record.supported = supported;
        }

        if let Some(active) = self.active {
            record.record_metadata.active = active;
        }

        record
    }

    fn get_store(stores: AppStores) -> MongoStore<Self::Output> {
        stores.model_config.clone()
    }
}
