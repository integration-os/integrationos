use super::{create, delete, read, update, CrudHook, CrudRequest, Unit};
use crate::{
    api_payloads::ErrorResponse,
    internal_server_error, not_found,
    server::{AppState, AppStores},
    service_unavailable,
};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{patch, post},
    Extension, Json, Router,
};
use bson::SerializerOptions;
use chrono::Utc;
use http::HeaderMap;
use integrationos_domain::{
    algebra::{MongoStore, StoreExt},
    api_model_config::{
        ApiModelConfig, AuthMethod, ModelPaths, ResponseBody, SamplesInput, SchemasInput,
    },
    connection_model_definition::{
        ConnectionModelDefinition, CrudAction, CrudMapping, ExtractorConfig, PlatformInfo,
        TestConnection, TestConnectionState,
    },
    event_access::EventAccess,
    get_secret_request::GetSecretRequest,
    id::{prefix::IdPrefix, Id},
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
    Extension(event_access): Extension<Arc<EventAccess>>,
    Path(id): Path<String>,
    State(state): State<Arc<AppState>>,
    Json(req): Json<TestConnectionPayload>,
) -> Result<Json<TestConnectionResponse>, (StatusCode, Json<ErrorResponse>)> {
    let connection = match state
        .app_stores
        .connection
        .get_one(doc! {
            "key": req.connection_key,
            "ownership.buildableId": event_access.ownership.id.as_ref(),
            "deleted": false
        })
        .await
    {
        Ok(Some(data)) => data,
        Ok(None) => {
            return Err(not_found!("Connection Record"));
        }
        Err(e) => {
            error!("Error fetching connection in testing endpoint: {:?}", e);

            return Err(internal_server_error!());
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
            return Err(not_found!("Inactive Connection Model Definition Record"));
        }
        Err(e) => {
            error!(
                "Error fetching inactive connection model definition in testing endpoint: {:?}",
                e
            );

            return Err(internal_server_error!());
        }
    };

    let mut secret_result = state
        .secrets_client
        .decrypt(&GetSecretRequest {
            buildable_id: connection.ownership.id.to_string(),
            id: connection.secrets_service_id.clone(),
        })
        .await
        .map_err(|e| {
            error!("Error decripting secret for connection: {:?}", e);

            internal_server_error!()
        })?;

    let request_string: String = serde_json::to_string(&req.request.clone()).map_err(|e| {
        error!(
            "Error converting request to json string in testing endpoint: {:?}",
            e
        );

        internal_server_error!()
    })?;

    // Add path params to template context
    if let Some(path_params) = req.request.path_params {
        for (key, val) in path_params {
            secret_result[key] = Value::String(val);
        }
    }

    let request_body_vec = req.request.body.map(|body| body.to_string().into_bytes());
    let model_execution_result = state
        .extractor_caller
        .execute_model_definition(
            &Arc::new(connection_model_definition.clone()),
            req.request.headers.unwrap_or_default(),
            &req.request.query_params.unwrap_or(HashMap::new()),
            &Arc::new(secret_result),
            request_body_vec,
        )
        .await
        .map_err(|e| {
            error!("Error executing connection model definition: {:?}", e);

            service_unavailable!()
        })?;

    let status_code = model_execution_result.status();

    let response_body = model_execution_result.text().await.map_err(|e| {
        error!("Could not get text from test connection failure: {e}");
        service_unavailable!()
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

    let status_bson = bson::to_bson_with_options(
        &status,
        SerializerOptions::builder().human_readable(false).build(),
    )
    .map_err(|e| {
        error!("Error serializing status to BSON: {:?}", e);
        internal_server_error!()
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

            internal_server_error!()
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

    Ok(Json(response))
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[cfg_attr(feature = "dummy", derive(fake::Dummy))]
#[serde(rename_all = "camelCase")]
pub struct CreateRequest {
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
}

impl CrudHook<ConnectionModelDefinition> for CreateRequest {}

impl CrudRequest for CreateRequest {
    type Output = ConnectionModelDefinition;

    fn filterable() -> bool {
        false
    }

    fn output(&self) -> Option<Self::Output> {
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
            id: Id::new(IdPrefix::ConnectionModelDefinition, Utc::now()),
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
        };
        record.record_metadata.version = self.version.clone();
        Some(record)
    }

    fn update(&self, record: &mut Self::Output) -> Unit {
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
        record.connection_platform = self.connection_platform.clone();
        record.connection_definition_id = self.connection_definition_id;
        record.platform_version = self.platform_version.clone();
        record.title = self.title.clone();
        record.name = self.name.clone();
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
        record.mapping = self.mapping.clone();
        record.extractor_config = self.extractor_config.clone();
        record.record_metadata.version = self.version.clone();
    }

    fn get_store(stores: AppStores) -> MongoStore<Self::Output> {
        stores.model_config.clone()
    }
}
