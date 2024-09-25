use super::event_access::CreateEventAccessPayloadWithOwnership;
use crate::{logic::event_access::get_client_throughput, server::AppState};
use axum::{
    extract::{Path, State},
    routing::post,
    Extension, Json, Router,
};
use chrono::{Duration, Utc};
use http::{HeaderMap, HeaderName, HeaderValue};
use integrationos_domain::{
    algebra::{MongoStore, TemplateExt},
    api_model_config::ContentType,
    connection_definition::ConnectionDefinition,
    connection_oauth_definition::{
        Computation, ConnectionOAuthDefinition, OAuthResponse, PlatformSecret, Settings,
    },
    environment::Environment,
    event_access::EventAccess,
    id::{prefix::IdPrefix, Id},
    oauth_secret::OAuthSecret,
    ownership::Ownership,
    ApplicationError, Connection, ConnectionIdentityType, ErrorMeta, IntegrationOSError,
    InternalError, OAuth, Throughput,
};
use mongodb::bson::doc;
use reqwest::Request;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::{to_string_pretty, Value};
use std::{
    collections::{BTreeMap, HashMap},
    str::FromStr,
    sync::Arc,
};
use tracing::{debug, error};
use uuid::Uuid;

pub fn get_router() -> Router<Arc<AppState>> {
    Router::new().route("/:platform", post(oauth_handler))
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[cfg_attr(feature = "dummy", derive(fake::Dummy))]
#[serde(rename_all = "camelCase")]
struct OAuthRequest {
    #[serde(rename = "__isEngineeringAccount__", default)]
    is_engineering_account: bool,
    connection_definition_id: Id,
    client_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    payload: Option<Value>,
    identity: Option<String>,
    identity_type: Option<ConnectionIdentityType>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[cfg_attr(feature = "dummy", derive(fake::Dummy))]
#[serde(rename_all = "camelCase")]
struct OAuthPayload {
    client_id: String,
    client_secret: String,
    metadata: Value,
}

impl OAuthPayload {
    fn as_json(&self) -> Option<Value> {
        serde_json::to_value(self).ok()
    }
}

async fn oauth_handler(
    state: State<Arc<AppState>>,
    Extension(user_event_access): Extension<Arc<EventAccess>>,
    Path(platform): Path<String>,
    Json(payload): Json<OAuthRequest>,
) -> Result<Json<Connection>, IntegrationOSError> {
    let conn_oauth_definition = get_conn_oauth_definition(&state, &platform).await?;
    let setting = get_user_settings(
        &state,
        &user_event_access.ownership,
        payload.is_engineering_account,
    )
    .await
    .map_err(|e| {
        error!("Failed to get user settings: {:?}", e);
        e
    })?;

    let environment = user_event_access.environment;

    let secret = get_secret::<PlatformSecret>(
        &state,
        setting
            .platform_secret(&payload.connection_definition_id, environment)
            .ok_or_else(|| {
                error!("Settings does not have a secret service id for the connection platform");
                InternalError::invalid_argument(
                    "Provided connection definition does not have a secret entry",
                    None,
                )
            })?,
        if payload.is_engineering_account {
            tracing::info!("Using engineering account id for secret");
            state.config.engineering_account_id.clone()
        } else {
            tracing::info!("Using user event access id for secret");
            user_event_access.clone().ownership.id.to_string()
        },
    )
    .await
    .map_err(|e| {
        error!("Failed to get platform secret for connection: {:?}", e);
        e
    })?;

    let mut oauth_payload = OAuthPayload {
        metadata: payload.payload.clone().unwrap_or(Value::Null),
        client_id: payload.client_id,
        client_secret: secret.client_secret,
    };

    if let Some(metadata) = oauth_payload.metadata.as_object_mut() {
        metadata.insert(
            "environment".to_string(),
            Value::String(environment.to_string()),
        );
    }

    let conn_oauth_definition = if conn_oauth_definition.is_full_template_enabled {
        state
            .template
            .render_as(&conn_oauth_definition, oauth_payload.as_json().as_ref())
            .map_err(|e| {
                error!("Failed to render oauth definition: {:?}", e);
                e
            })?
    } else {
        conn_oauth_definition
    };

    let request =
        request(&conn_oauth_definition, &oauth_payload, &state.template).map_err(|e| {
            error!("Failed to create oauth request: {}", e);
            e
        })?;

    debug!("Request: {:?}", request);
    let response = state
        .http_client
        .execute(request)
        .await
        .map(|response| response.json::<Value>())
        .map_err(|e| {
            error!("Failed to execute oauth request: {}", e);
            InternalError::script_error(&e.to_string(), None)
        })?
        .await
        .map_err(|e| {
            error!("Failed to decode third party oauth response: {:?}", e);
            InternalError::deserialize_error(&e.to_string(), None)
        })?;

    debug!("Response: {:?}", response);

    let decoded: OAuthResponse = conn_oauth_definition
        .compute
        .init
        .response
        .compute(&response)
        .map_err(|e| {
            error!("Failed to decode oauth response: {:?}", e);
            InternalError::script_error(e.message().as_ref(), None)
        })?;

    let oauth_secret = OAuthSecret::from_init(
        decoded,
        oauth_payload.client_id,
        oauth_payload.client_secret,
        response,
        payload.payload,
    );

    let secret = state
        .secrets_client
        .create(
            &oauth_secret.as_json(),
            user_event_access.clone().ownership.id.as_ref(),
        )
        .await
        .map_err(|e| {
            error!("Failed to create oauth secret: {}", e);
            InternalError::encryption_error(e.message().as_ref(), None)
        })?;

    let conn_definition = get_conn_definition(&state, &payload.connection_definition_id).await?;
    let group = Uuid::new_v4().to_string().replace('-', "");
    let namespace = "default".to_string();

    let key = format!(
        "{}::{}::{}::{}",
        user_event_access.environment, conn_definition.platform, namespace, group
    );

    let throughput = get_client_throughput(&user_event_access.ownership.id, &state).await?;

    let event_access = CreateEventAccessPayloadWithOwnership {
        name: format!(
            "{} {} account",
            user_event_access.environment, conn_definition.name
        ),
        group: Some(group.clone()),
        platform: conn_definition.platform.clone(),
        namespace: None,
        connection_type: conn_definition.r#type.clone(),
        environment: user_event_access.environment,
        paths: conn_definition.paths.clone(),
        ownership: user_event_access.ownership.clone(),
        throughput: Some(throughput),
    }
    .as_event_access(&state.config)
    .map_err(|e| {
        error!("Error creating event access for oauth connection: {:?}", e);
        ApplicationError::service_unavailable("Failed to create event access", None)
    })?;

    state
        .app_stores
        .event_access
        .create_one(&event_access)
        .await
        .map_err(|e| {
            error!("Error saving event access for oauth connection: {:?}", e);
            e
        })?;

    let connection = Connection {
        id: Id::new(IdPrefix::Connection, Utc::now()),
        platform_version: conn_definition.clone().platform_version,
        connection_definition_id: conn_definition.id,
        r#type: conn_definition.to_connection_type(),
        group,
        key: key.clone().into(),
        environment: user_event_access.environment,
        platform: platform.into(),
        secrets_service_id: secret.id(),
        event_access_id: event_access.id,
        access_key: event_access.access_key,
        identity: payload.identity,
        identity_type: payload.identity_type,
        settings: conn_definition.settings,
        throughput: Throughput {
            key,
            limit: throughput,
        },
        ownership: user_event_access.ownership.clone(),
        oauth: Some(OAuth::Enabled {
            connection_oauth_definition_id: conn_oauth_definition.id,
            expires_in: Some(oauth_secret.expires_in),
            expires_at: Some(
                chrono::Utc::now()
                    .checked_add_signed(Duration::seconds(oauth_secret.expires_in as i64))
                    .unwrap_or_else(chrono::Utc::now)
                    .checked_sub_signed(Duration::seconds(120))
                    .unwrap_or_else(chrono::Utc::now)
                    .timestamp(),
            ),
        }),
        record_metadata: Default::default(),
    };

    state
        .app_stores
        .connection
        .create_one(&connection)
        .await
        .map_err(|e| {
            error!("Failed to create connection: {}", e);
            ApplicationError::service_unavailable("Failed to create connection", None)
        })?;

    Ok(Json(connection))
}

fn request(
    oauth_definition: &ConnectionOAuthDefinition,
    payload: &OAuthPayload,
    template: &impl TemplateExt,
) -> Result<Request, IntegrationOSError> {
    let payload = serde_json::to_value(payload).map_err(|e| {
        error!("Failed to serialize oauth payload: {}", e);
        InternalError::serialize_error(&e.to_string(), None)
    })?;
    let computation = oauth_definition
        .compute
        .init
        .computation
        .clone()
        .map(|computation| computation.compute::<Computation>(&payload))
        .transpose()
        .map_err(|e| {
            error!("Failed to compute oauth payload: {}", e);
            InternalError::script_error(e.message().as_ref(), None)
        })?;

    let headers = header(oauth_definition, computation.as_ref(), template)?;
    let query = query(oauth_definition, computation.as_ref(), template)?;
    let body = body(&payload, computation.as_ref(), template)?;

    let request = reqwest::Client::new()
        .post(oauth_definition.configuration.init.uri())
        .headers(headers);

    let request = match oauth_definition.configuration.init.content {
        Some(ContentType::Json) => request.json(&body).query(&query),
        Some(ContentType::Form) => request.form(&body).query(&query),
        _ => request.query(&query),
    };

    request.build().map_err(|e| {
        error!("Failed to build static request: {}", e);
        InternalError::unknown(&e.to_string(), None)
    })
}

fn query(
    oauth_definition: &ConnectionOAuthDefinition,
    computation: Option<&Computation>,
    template: &impl TemplateExt,
) -> Result<Option<Value>, IntegrationOSError> {
    let query_params = oauth_definition
        .configuration
        .init
        .query_params
        .as_ref()
        .map(|query_params| {
            let mut map = HashMap::new();
            for (key, value) in query_params {
                let key = key.to_string();
                let value = value.as_str();

                map.insert(key, value.to_string());
            }
            map
        });

    match query_params {
        Some(query_params) => {
            let payload = computation.and_then(|computation| computation.clone().query_params);

            let query_params_str = to_string_pretty(&query_params).map_err(|e| {
                error!("Failed to serialize query params: {}", e);
                InternalError::serialize_error(&e.to_string(), None)
            })?;

            let query_params = template.render(&query_params_str, payload.as_ref())?;

            let query_params: BTreeMap<String, String> = serde_json::from_str(&query_params)
                .map_err(|e| {
                    error!("Failed to deserialize query params: {}", e);
                    InternalError::deserialize_error(&e.to_string(), None)
                })?;

            Ok(Some(serde_json::to_value(query_params).map_err(|e| {
                error!("Failed to serialize query params: {}", e);
                InternalError::serialize_error(&e.to_string(), None)
            })?))
        }
        None => Ok(None),
    }
}

fn body(
    payload: &Value,
    computation: Option<&Computation>,
    template: &impl TemplateExt,
) -> Result<Option<Value>, IntegrationOSError> {
    let body = computation.and_then(|computation| computation.clone().body);

    match body {
        Some(body) => {
            let body_str = to_string_pretty(&body).map_err(|e| {
                error!("Failed to serialize body: {}", e);
                InternalError::serialize_error(&e.to_string(), None)
            })?;

            let body = template.render(&body_str, Some(payload))?;

            Ok(Some(serde_json::from_str(&body).map_err(|e| {
                error!("Failed to deserialize body: {}", e);
                InternalError::deserialize_error(&e.to_string(), None)
            })?))
        }
        None => Ok(None),
    }
}

fn header(
    conn_oauth_definition: &ConnectionOAuthDefinition,
    computation: Option<&Computation>,
    template: &impl TemplateExt,
) -> Result<HeaderMap, IntegrationOSError> {
    let headers = conn_oauth_definition
        .configuration
        .init
        .headers
        .as_ref()
        .and_then(|headers| {
            let mut map = HashMap::new();
            for (key, value) in headers {
                let key = key.to_string();
                let value = value.to_str().ok()?;

                map.insert(key, value.to_string());
            }
            Some(map)
        });

    match headers {
        Some(headers) => {
            let payload = computation.and_then(|computation| computation.clone().headers);

            let headers_str = to_string_pretty(&headers).map_err(|e| {
                error!("Failed to serialize headers: {}", e);
                InternalError::serialize_error(&e.to_string(), None)
            })?;

            let headers = template.render(&headers_str, payload.as_ref())?;

            let headers: BTreeMap<String, String> =
                serde_json::from_str(&headers).map_err(|e| {
                    error!("Failed to deserialize headers: {}", e);
                    InternalError::deserialize_error(&e.to_string(), None)
                })?;

            headers
                .iter()
                .try_fold(HeaderMap::new(), |mut header_map, (key, value)| {
                    let key = HeaderName::from_str(key).map_err(|e| {
                        error!("Failed to parse header name: {}", e);
                        InternalError::invalid_argument(&e.to_string(), None)
                    })?;

                    let value = HeaderValue::from_str(value).map_err(|e| {
                        error!("Failed to parse header value: {}", e);
                        InternalError::invalid_argument(&e.to_string(), None)
                    })?;

                    header_map.insert(key, value);

                    Ok(header_map)
                })
        }
        None => Ok(HeaderMap::new()),
    }
}

async fn get_conn_definition(
    state: &State<Arc<AppState>>,
    conn_definition_id: &Id,
) -> Result<ConnectionDefinition, IntegrationOSError> {
    let conn_definition_store: &MongoStore<ConnectionDefinition> =
        &state.app_stores.connection_config;

    let conn_definition: ConnectionDefinition = conn_definition_store
        .get_one(doc! {"_id": &conn_definition_id.to_string()})
        .await?
        .ok_or_else(|| ApplicationError::not_found("Connection definition", None))?;

    Ok(conn_definition)
}

async fn get_conn_oauth_definition(
    state: &State<Arc<AppState>>,
    platform: &str,
) -> Result<ConnectionOAuthDefinition, IntegrationOSError> {
    let oauth_definition_store: &MongoStore<ConnectionOAuthDefinition> =
        &state.app_stores.oauth_config;

    let conn_oauth_definition: ConnectionOAuthDefinition = oauth_definition_store
        .get_one(doc! {"connectionPlatform": &platform})
        .await?
        .ok_or_else(|| ApplicationError::not_found("Connection OAuth definition", None))?;

    Ok(conn_oauth_definition)
}

pub async fn get_user_settings(
    state: &State<Arc<AppState>>,
    ownership: &Ownership,
    is_engineering_account: bool,
) -> Result<Settings, IntegrationOSError> {
    let settings_store: &MongoStore<Settings> = &state.app_stores.settings;

    let ownership_id = if is_engineering_account {
        state.config.engineering_account_id.clone()
    } else {
        ownership.id.to_string()
    };

    let setting: Settings = settings_store
        .get_one(doc! {"ownership.buildableId": &ownership_id})
        .await?
        .ok_or_else(|| ApplicationError::not_found("Settings", None))?;

    Ok(setting)
}

async fn get_secret<S: DeserializeOwned>(
    state: &State<Arc<AppState>>,
    id: String,
    buildable_id: String,
) -> Result<S, IntegrationOSError> {
    let secrets_client = &state.secrets_client;

    let encoded_secret = secrets_client.get(&id, &buildable_id).await?;

    encoded_secret.decode::<S>()
}
