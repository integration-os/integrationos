use super::event_access::CreateEventAccessPayloadWithOwnership;
use crate::{endpoints::ApiError, internal_server_error, not_found, server::AppState};
use axum::{
    extract::{Path, State},
    routing::post,
    Extension, Json, Router,
};
use chrono::{Duration, Utc};
use http::{HeaderMap, HeaderName, HeaderValue};
use integrationos_domain::{
    algebra::{MongoStore, StoreExt},
    get_secret_request::GetSecretRequest,
    id::{prefix::IdPrefix, Id},
    oauth_secret::OAuthSecret,
    {
        api_model_config::ContentType,
        connection_definition::ConnectionDefinition,
        connection_oauth_definition::{
            Computation, ConnectionOAuthDefinition, OAuthResponse, PlatformSecret, Settings,
        },
        event_access::EventAccess,
        ownership::Ownership,
        Connection, OAuth, Throughput,
    },
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
use tracing::error;

pub fn get_router() -> Router<Arc<AppState>> {
    Router::new().route("/:platform", post(oauth_handler))
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[cfg_attr(feature = "dummy", derive(fake::Dummy))]
#[serde(rename_all = "camelCase")]
struct OAuthRequest {
    connection_definition_id: Id,
    client_id: String,
    group: String,
    label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    payload: Option<Value>,
}

type ApiResult<T> = std::result::Result<T, ApiError>;

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[cfg_attr(feature = "dummy", derive(fake::Dummy))]
#[serde(rename_all = "camelCase")]
struct OAuthPayload {
    client_id: String,
    client_secret: String,
    metadata: Value,
}

// All of the debug statements are for debugging purposes, they won't reach production
// and will be removed in the next MR.
async fn oauth_handler(
    state: State<Arc<AppState>>,
    Extension(user_event_access): Extension<Arc<EventAccess>>,
    Path(platform): Path<String>,
    Json(payload): Json<OAuthRequest>,
) -> ApiResult<Json<Connection>> {
    let oauth_definition = find_oauth_definition(&state, &platform).await?;
    let setting = find_settings(&state, &user_event_access.ownership).await?;
    tracing::debug!("Setting ------------------------------------ {:?}", setting);

    let secret = get_secret::<PlatformSecret>(
        &state,
        GetSecretRequest {
            id: setting.platform_secret(&payload.connection_definition_id).ok_or_else(|| {
                error!("Settings does not have a secret service id for the connection platform");
                not_found!(
                    "Settings does not have a secret service id for the connection platform provided, settings"
                )
            })?,
            buildable_id: user_event_access.clone().ownership.id.to_string()
        },
    )
    .await?;

    let oauth_payload = OAuthPayload {
        metadata: payload.payload.clone().unwrap_or(Value::Null),
        client_id: payload.client_id,
        client_secret: secret.client_secret,
    };

    tracing::debug!(
        "OAuth Payload ------------------------------------ {:?}",
        oauth_payload
    );

    let request = request(&oauth_definition, &oauth_payload)?;
    let response = state
        .http_client
        .execute(request)
        .await
        .map(|response| response.json::<Value>())
        .map_err(|e| {
            error!("Failed to execute oauth request: {}", e);
            not_found!("Failed to execute oauth request {e}")
        })?
        .await
        .map_err(|e| {
            error!("Failed to decode third party oauth response: {}", e);
            internal_server_error!()
        })?;

    tracing::debug!("oauth response: {:?}", response);

    let decoded: OAuthResponse = oauth_definition
        .compute
        .init
        .response
        .compute(&response)
        .map_err(|e| {
            error!("Failed to decode oauth response: {}", e);
            internal_server_error!()
        })?;

    let oauth_secret = OAuthSecret::from_init(
        decoded,
        oauth_payload.client_id,
        oauth_payload.client_secret,
        response,
        payload.payload,
    );

    tracing::debug!(
        "OAuth Secret ------------------------------------ {:?}",
        oauth_secret
    );

    let secret = state
        .secrets_client
        .encrypt(
            user_event_access.clone().ownership.id.to_string(),
            &oauth_secret.as_json(),
        )
        .await
        .map_err(|e| {
            error!("Failed to create oauth secret: {}", e);
            internal_server_error!()
        })?;

    let connection_definition =
        find_connection_definition(&state, &payload.connection_definition_id).await?;

    let key = format!(
        "{}::{}::{}",
        user_event_access.environment, connection_definition.platform, payload.group
    );

    let event_access = CreateEventAccessPayloadWithOwnership {
        name: payload.label.clone(),
        group: Some(payload.group.clone()),
        platform: connection_definition.platform.clone(),
        namespace: None,
        connection_type: connection_definition.r#type.clone(),
        environment: user_event_access.environment,
        paths: connection_definition.paths.clone(),
        ownership: user_event_access.ownership.clone(),
    }
    .as_event_access(&state.config)
    .map_err(|e| {
        error!("Error creating event access for connection: {:?}", e);

        internal_server_error!()
    })?;

    let connection = Connection {
        id: Id::new(IdPrefix::Connection, Utc::now()),
        platform_version: connection_definition.clone().platform_version,
        connection_definition_id: connection_definition.id,
        r#type: connection_definition.to_connection_type(),
        name: payload.label,
        key: key.clone().into(),
        group: payload.group,
        environment: user_event_access.environment,
        platform: platform.into(),
        secrets_service_id: secret.id,
        event_access_id: event_access.id,
        access_key: event_access.access_key,
        settings: connection_definition.settings,
        throughput: Throughput { key, limit: 100 },
        ownership: user_event_access.ownership.clone(),
        oauth: Some(OAuth::Enabled {
            connection_oauth_definition_id: oauth_definition.id,
            expires_in: Some(oauth_secret.expires_in),
            expires_at: Some(
                (chrono::Utc::now()
                    + Duration::try_seconds(oauth_secret.expires_in as i64)
                        .unwrap_or(Duration::zero()))
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
            internal_server_error!()
        })?;

    Ok(Json(connection))
}

fn request(
    oauth_definition: &ConnectionOAuthDefinition,
    payload: &OAuthPayload,
) -> ApiResult<Request> {
    let payload = serde_json::to_value(payload).map_err(|e| {
        error!("Failed to serialize oauth payload: {}", e);
        internal_server_error!()
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
            internal_server_error!()
        })?;

    let headers = header(oauth_definition, computation.as_ref())?;
    let query = query(oauth_definition, computation.as_ref())?;
    let body = body(&payload, computation.as_ref())?;

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
        internal_server_error!()
    })
}

fn query(
    oauth_definition: &ConnectionOAuthDefinition,
    computation: Option<&Computation>,
) -> ApiResult<Option<Value>> {
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

            let handlebars = handlebars::Handlebars::new();

            let query_params_str = to_string_pretty(&query_params).map_err(|e| {
                error!("Failed to serialize query params: {}", e);
                internal_server_error!()
            })?;

            let query_params = handlebars
                .render_template(&query_params_str, &payload)
                .map_err(|e| {
                    error!("Failed to render query params: {}", e);
                    internal_server_error!()
                })?;

            let query_params: BTreeMap<String, String> = serde_json::from_str(&query_params)
                .map_err(|e| {
                    error!("Failed to deserialize query params: {}", e);
                    internal_server_error!()
                })?;

            Ok(Some(serde_json::to_value(query_params).map_err(|e| {
                error!("Failed to serialize query params: {}", e);
                internal_server_error!()
            })?))
        }
        None => Ok(None),
    }
}

fn body(payload: &Value, computation: Option<&Computation>) -> ApiResult<Option<Value>> {
    let body = computation.and_then(|computation| computation.clone().body);

    match body {
        Some(body) => {
            let handlebars = handlebars::Handlebars::new();

            let body_str = to_string_pretty(&body).map_err(|e| {
                error!("Failed to serialize body: {}", e);
                internal_server_error!()
            })?;

            let body = handlebars
                .render_template(&body_str, &payload)
                .map_err(|e| {
                    error!("Failed to render body: {}", e);
                    internal_server_error!()
                })?;

            Ok(Some(serde_json::from_str(&body).map_err(|e| {
                error!("Failed to deserialize body: {}", e);
                internal_server_error!()
            })?))
        }
        None => Ok(None),
    }
}

fn header(
    oauth_definition: &ConnectionOAuthDefinition,
    computation: Option<&Computation>,
) -> ApiResult<HeaderMap> {
    let headers = oauth_definition
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

            let handlebars = handlebars::Handlebars::new();

            let headers_str = to_string_pretty(&headers).map_err(|e| {
                error!("Failed to serialize headers: {}", e);
                internal_server_error!()
            })?;

            let headers = handlebars
                .render_template(&headers_str, &payload)
                .map_err(|e| {
                    error!("Failed to render headers: {}", e);
                    internal_server_error!()
                })?;

            let headers: BTreeMap<String, String> =
                serde_json::from_str(&headers).map_err(|e| {
                    error!("Failed to deserialize headers: {}", e);
                    internal_server_error!()
                })?;

            headers
                .iter()
                .try_fold(HeaderMap::new(), |mut header_map, (key, value)| {
                    let key = HeaderName::from_str(key).map_err(|e| {
                        error!("Failed to parse header name: {}", e);
                        internal_server_error!()
                    })?;

                    let value = HeaderValue::from_str(value).map_err(|e| {
                        error!("Failed to parse header value: {}", e);
                        internal_server_error!()
                    })?;

                    header_map.insert(key, value);

                    Ok(header_map)
                })
        }
        None => Ok(HeaderMap::new()),
    }
}

async fn find_connection_definition(
    state: &State<Arc<AppState>>,
    connection_definition_id: &Id,
) -> ApiResult<ConnectionDefinition> {
    let connection_definition_store: &MongoStore<ConnectionDefinition> =
        &state.app_stores.connection_config;

    let connection_definition: ConnectionDefinition = connection_definition_store
        .get_one(doc! {"_id": &connection_definition_id.to_string()})
        .await
        .map_err(|e| {
            error!("Failed to retrieve connection definition: {}", e);
            internal_server_error!()
        })?
        .ok_or_else(|| not_found!("Connection definition not found"))?;

    Ok(connection_definition)
}

async fn find_oauth_definition(
    state: &State<Arc<AppState>>,
    platform: &str,
) -> ApiResult<ConnectionOAuthDefinition> {
    let oauth_definition_store: &MongoStore<ConnectionOAuthDefinition> =
        &state.app_stores.oauth_config;

    let oauth_definition: ConnectionOAuthDefinition = oauth_definition_store
        .get_one(doc! {"connectionPlatform": &platform})
        .await
        .map_err(|e| {
            error!("Failed to find oauth definition: {}", e);
            internal_server_error!()
        })?
        .ok_or_else(|| not_found!("Oauth definition"))?;

    Ok(oauth_definition)
}

async fn find_settings(state: &State<Arc<AppState>>, ownership: &Ownership) -> ApiResult<Settings> {
    let settings_store: &MongoStore<Settings> = &state.app_stores.settings;

    let setting: Settings = settings_store
        .get_one(doc! {"ownership.buildableId": &ownership.id.to_string()})
        .await
        .map_err(|e| {
            error!("Failed to retrieve from settings store: {}", e);
            internal_server_error!()
        })?
        .ok_or_else(|| not_found!("Settings"))?;

    Ok(setting)
}

async fn get_secret<S: DeserializeOwned>(
    state: &State<Arc<AppState>>,
    get_secret_request: GetSecretRequest,
) -> ApiResult<S> {
    let secrets_client = &state.secrets_client;

    let encoded_secret = secrets_client
        .decrypt(&get_secret_request)
        .await
        .map_err(|e| {
            error!("Failed to retrieve oauth secret: {}", e);
            internal_server_error!()
        })?;

    serde_json::from_value::<S>(encoded_secret).map_err(|e| {
        error!("Failed to deserialize owner secret: {}", e);
        internal_server_error!()
    })
}
