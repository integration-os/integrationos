use crate::{
    config::Config, finalizer::event::FinalizeEvent, mock::finalizer::MockFinalizer,
    util::get_value_from_path,
};
use anyhow::{anyhow, Result};
use axum::{
    body::Bytes,
    debug_handler,
    extract::{Path, Query, State},
    http::{HeaderMap, HeaderName, StatusCode},
    routing::{get, post},
    Json, Router,
};
use axum_prometheus::PrometheusMetricLayer;
use integrationos_domain::{
    encrypted_access_key::EncryptedAccessKey, encrypted_data::PASSWORD_LENGTH,
    event_response::EventResponse, event_type::EventType, AccessKey, Event,
};
use moka::future::Cache;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{collections::HashMap, iter::once, sync::Arc};
use tokio::net::TcpListener;
use tower_http::{
    cors::{Any, CorsLayer},
    sensitive_headers::SetSensitiveRequestHeadersLayer,
    trace::TraceLayer,
};
use tracing::{error, info, warn};

const HEADER_STR: &str = "x-buildable-secret";
const INVALID_ACCESS_KEY_ERROR: (StatusCode, &str) =
    (StatusCode::BAD_REQUEST, "Invalid access key");
const MISSING_HEADER_ERROR: (StatusCode, &str) =
    (StatusCode::BAD_REQUEST, "Missing x-buildable-secret header");

pub struct AppState {
    pub config: Config,
    pub cache: Cache<EncryptedAccessKey<'static>, AccessKey>,
    pub finalizer: Arc<dyn FinalizeEvent + Sync + Send>,
}

impl AppState {
    pub fn new(config: Config, finalizer: Arc<dyn FinalizeEvent + Sync + Send>) -> Self {
        let cache = Cache::new(config.cache_size);
        Self {
            config,
            cache,
            finalizer,
        }
    }

    pub fn get_secret_key(&self) -> [u8; PASSWORD_LENGTH] {
        // We validate that the config must have 32 byte secret key in main.rs
        // So this is safe to unwrap
        self.config.secret_key.as_bytes().try_into().unwrap()
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct EventRequest {
    pub event: String,
    pub payload: Value,
}

#[derive(Clone)]
pub struct Server {
    config: Config,
    finalizer: Arc<dyn FinalizeEvent + Sync + Send>,
}

impl Default for Server {
    fn default() -> Self {
        Self {
            config: Config::default(),
            finalizer: Arc::new(MockFinalizer),
        }
    }
}

impl Server {
    pub fn new(config: Config, finalizer: impl FinalizeEvent + Sync + Send + 'static) -> Self {
        Self {
            config,
            finalizer: Arc::new(finalizer),
        }
    }

    pub async fn run(&self) -> Result<()> {
        let app = self.get_router();
        info!("Gateway server listening on {}", self.config.address);

        let tcp_listener = TcpListener::bind(&self.config.address)
            .await
            .map_err(|e| anyhow!("Failed to bind to address: {}", e))?;

        axum::serve(tcp_listener, app.into_make_service())
            .await
            .map_err(|e| anyhow!("Server error: {}", e))
    }

    fn get_router(&self) -> Router {
        let state = Arc::new(AppState::new(self.config.clone(), self.finalizer.clone()));
        let mut router = Router::new()
            .route("/emit", post(post_event_sk))
            .route("/emit/:id", post(post_event_id))
            .layer(SetSensitiveRequestHeadersLayer::new(once(
                HeaderName::from_lowercase(HEADER_STR.as_bytes()).unwrap(),
            )))
            .layer(TraceLayer::new_for_http())
            .route("/", get(get_root))
            .layer(CorsLayer::new().allow_origin(Any))
            .with_state(state);

        if !cfg!(test) {
            let (prometheus_layer, metric_handle) = PrometheusMetricLayer::pair();
            router = router
                .route("/metrics", get(|| async move { metric_handle.render() }))
                .layer(prometheus_layer);
        }
        router
    }

    pub async fn handle_event(
        encrypted_access_key: EncryptedAccessKey<'_>,
        payload: Bytes,
        query: Option<HashMap<String, String>>,
        headers: HeaderMap,
        state: Arc<AppState>,
    ) -> Result<Json<EventResponse>, (StatusCode, &'static str)> {
        if encrypted_access_key.prefix.environment != state.config.environment {
            warn!("Identifier is wrong environment");
            return Err(INVALID_ACCESS_KEY_ERROR);
        };

        let encrypted_access_key = encrypted_access_key.to_static();
        let access_key = if let Some(access_key) = state.cache.get(&encrypted_access_key).await {
            access_key
        } else if let Ok(access_key) =
            AccessKey::parse(&encrypted_access_key, &state.get_secret_key())
        {
            state
                .cache
                .insert(encrypted_access_key.clone().to_static(), access_key.clone())
                .await;
            access_key
        } else {
            warn!("Identifier cannot be decrypted");
            return Err(INVALID_ACCESS_KEY_ERROR);
        };

        let (name, payload) = if access_key.prefix.event_type == EventType::SecretKey {
            let payload = match serde_json::from_slice::<EventRequest>(&payload) {
                Ok(payload) => payload,
                Err(e) => {
                    warn!("Failed to deserialize payload: {e:?}");
                    return Err((StatusCode::BAD_REQUEST, "Failed to deserialize payload"));
                }
            };
            (payload.event, payload.payload.to_string())
        } else {
            let name = get_value_from_path(
                &mut access_key.data.event_path.clone(),
                &headers,
                &payload,
                &query,
            )
            .unwrap_or("null".to_string());
            let payload = match String::from_utf8(payload.to_vec()) {
                Ok(payload) => payload,
                Err(e) => {
                    warn!("Failed to deserialize payload: {e:?}");
                    return Err((StatusCode::BAD_REQUEST, "Failed to deserialize payload"));
                }
            };
            (name, payload)
        };

        let event = Event::new(&access_key, &encrypted_access_key, &name, headers, payload);

        match state
            .finalizer
            .finalize_event(&event, &name, &encrypted_access_key)
            .await
        {
            Ok(_) => Ok(Json(EventResponse::new(event))),
            Err(e) => {
                error!("Failed to finalize event: {e:?}");
                Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Failed to acknowledge event",
                ))
            }
        }
    }
}

#[debug_handler]
async fn post_event_sk(
    headers: HeaderMap,
    State(state): State<Arc<AppState>>,
    body: Bytes,
) -> Result<Json<EventResponse>, (StatusCode, &'static str)> {
    let Some(identifier) = headers.get(HEADER_STR) else {
        return Err(MISSING_HEADER_ERROR);
    };

    let Ok(identifier) = identifier.to_str() else {
        warn!("Could not convert identifier to string");
        return Err(INVALID_ACCESS_KEY_ERROR);
    };

    let encrypted_key = match EncryptedAccessKey::parse(identifier) {
        Ok(e) => e,
        Err(e) => {
            warn!("Could not parse identifier: {e}");
            return Err(INVALID_ACCESS_KEY_ERROR);
        }
    };

    if encrypted_key.prefix.event_type != EventType::SecretKey {
        warn!("Identifier is not type \"secret key\"");
        return Err(INVALID_ACCESS_KEY_ERROR);
    }

    Server::handle_event(encrypted_key, body, None, headers.clone(), state).await
}

#[debug_handler]
async fn post_event_id(
    headers: HeaderMap,
    query: Option<Query<HashMap<String, String>>>,
    Path(identifier): Path<String>,
    State(state): State<Arc<AppState>>,
    body: Bytes,
) -> Result<Json<EventResponse>, (StatusCode, &'static str)> {
    let encrypted_key = match EncryptedAccessKey::parse(&identifier) {
        Ok(e) => e,
        Err(e) => {
            warn!("Could not parse identifier: {e}");
            return Err(INVALID_ACCESS_KEY_ERROR);
        }
    };

    if encrypted_key.prefix.event_type != EventType::Id {
        warn!("Identifier is not type \"id\"");
        return Err(INVALID_ACCESS_KEY_ERROR);
    }

    let query = query.map(|q| q.0);

    Server::handle_event(encrypted_key, body, query, headers, state).await
}

async fn get_root() {}

#[cfg(test)]
mod tests {
    use axum::{
        body::Body,
        http::{header::CONTENT_TYPE, Method, Request, StatusCode},
    };
    use http_body_util::BodyExt;
    use integrationos_domain::{
        event_state::EventState,
        hashes::{HashType, HashValue},
    };
    use tower::ServiceExt;

    use super::*;

    const VALID_ID_KEY: &str = "id_test_1_Q71YUIZydcgSwJQNOUCHhaTMqmIvslIafF5LluORJfJKydMGELHtYe_ydtBIrVuomEnOZ4jfZQgtkqWxtG-s7vhbyir4kNjLyHKyDyh1SDubBMlhSI7Mq-M5RVtwnwFqZiOeUkIgHJFgcGQn0Plb1AkAAAAAAAAAAAAAAAAAAAAAAMwWY_9_oDOV75noniBViOVmVPUQqzcW8G3P8nuUD6Q";
    const VALID_SK_KEY: &str = "sk_test_1_Q71YUIZydcgSwJQNOUCHhaTMqmIvslIafF5LluORJfJKydMGELHtYe_ydtBIrVuomEnOZ4jfZQgtkqWxtG-s7vhbyir4kNjLyHKyDyh1SDubBMlhSI7Mq-M5RVtwnwFqZiOeUkIgHJFgcGQn0Plb1AkAAAAAAAAAAAAAAAAAAAAAAMwWY_9_oDOV75noniBViOVmVPUQqzcW8G3P8nuUD6Q";

    #[tokio::test]
    async fn test_emit_id() {
        let router = Server::default().get_router();
        let response = router
            .oneshot(
                Request::builder()
                    .uri(format!("/emit/{VALID_ID_KEY}"))
                    .header(CONTENT_TYPE, "application/json")
                    .method(Method::POST)
                    .body(Body::from("{\"foo\": \"bar\"}"))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let resp = serde_json::from_slice::<EventResponse>(&body).unwrap();
        assert_eq!(resp.status, EventState::Acknowledged);
        assert_eq!(
            resp.hashes,
            [
                HashValue {
                    r#type: HashType::Body,
                    hash: "22c11adbd1e780c95a6840ea76c1d6727aba620cd41474c712129d1b22f5ea71"
                        .to_owned(),
                },
                HashValue {
                    r#type: HashType::Event,
                    hash: "40c5dcdc28d5bdad5346a822218b0ef0ef996427e411849d1c7f4df205b27060"
                        .to_owned(),
                },
                HashValue {
                    r#type: HashType::ModelBody,
                    hash: "848be1959192dc863543bd71428dad257c341e34b640d80ac9a4692b06e93db7"
                        .to_owned(),
                },
            ]
        );
    }

    #[tokio::test]
    async fn test_invalid_emit_id() {
        let router = Server::default().get_router();
        let response = router
            .oneshot(
                Request::builder()
                    .uri(format!("/emit/{VALID_SK_KEY}"))
                    .header(CONTENT_TYPE, "application/json")
                    .method(Method::POST)
                    .body(Body::from("{\"foo\": \"bar\"}"))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        assert_eq!(&body[..], b"Invalid access key");
    }

    #[tokio::test]
    async fn test_emit_sk() {
        let router = Server::default().get_router();
        let response = router
            .oneshot(
                Request::builder()
                    .uri("/emit")
                    .header(CONTENT_TYPE, "application/json")
                    .header(HEADER_STR, VALID_SK_KEY)
                    .method(Method::POST)
                    .body(Body::from("{\"event\": \"foo\", \"payload\": \"bar\"}"))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let resp = serde_json::from_slice::<EventResponse>(&body).unwrap();
        assert_eq!(resp.status, EventState::Acknowledged);
        assert_eq!(
            resp.hashes,
            [
                HashValue {
                    r#type: HashType::Body,
                    hash: "2fe2fcbf5698e1ede94a12436044883d964c9d36ba32bee0d6ef69bd9e83bbad"
                        .to_owned(),
                },
                HashValue {
                    r#type: HashType::Event,
                    hash: "be1eef101bdd5dd790d31e23da5b41c551647bf82ffa6baf0c32b73b34a7a6f9"
                        .to_owned(),
                },
                HashValue {
                    r#type: HashType::ModelBody,
                    hash: "e63f132f3658d07b1b0de65d1c71bc95ec65e950b72d7743033f0450ff1e3bb6"
                        .to_owned(),
                },
            ]
        );
    }

    #[tokio::test]
    async fn test_invalid_emit_sk() {
        let router = Server::default().get_router();
        let response = router
            .oneshot(
                Request::builder()
                    .uri("/emit/123")
                    .header(CONTENT_TYPE, "application/json")
                    .header(HEADER_STR, VALID_ID_KEY)
                    .method(Method::POST)
                    .body(Body::from("{\"event\": \"foo\", \"payload\": \"bar\"}"))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        assert_eq!(body, "Invalid access key");
    }

    #[tokio::test]
    async fn test_root_returns_ok() {
        let router = Server::default().get_router();
        let response = router
            .oneshot(
                Request::builder()
                    .uri("/")
                    .method(Method::GET)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }
}
