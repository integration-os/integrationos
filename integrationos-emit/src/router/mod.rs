use crate::{logic::emitter, server::AppState};
use axum::{middleware::from_fn, response::IntoResponse, routing::get, Json, Router};
use chrono::Utc;
use http::StatusCode;
use integrationos_domain::{
    telemetry::log_request_middleware, Claims, IntegrationOSError, InternalError, DEFAULT_AUDIENCE,
    DEFAULT_ISSUER,
};
use jsonwebtoken::{EncodingKey, Header};
use serde_json::json;
use std::sync::Arc;
use tower_http::{cors::CorsLayer, trace::TraceLayer};

pub async fn get_router(state: &Arc<AppState>) -> Router<Arc<AppState>> {
    let path = format!("/{}", state.config.api_version);
    Router::new()
        .nest(&path, emitter::get_router())
        .route("/", get(get_root))
        .fallback(not_found_handler)
        .layer(CorsLayer::permissive())
        .layer(from_fn(log_request_middleware))
        .layer(TraceLayer::new_for_http())
}

pub async fn get_root() -> impl IntoResponse {
    Json(json!({ "success": true }))
}

pub async fn not_found_handler() -> impl IntoResponse {
    (
        StatusCode::NOT_FOUND,
        Json(json!({ "error": "Not found", })),
    )
}

///Generates a short live token for administrative requests to the API
pub fn generate_token(state: &AppState) -> Result<String, IntegrationOSError> {
    let now = Utc::now();

    let header = Header::default();
    let claims = Claims {
        is_buildable_core: true,
        iat: now.timestamp(),
        exp: now.timestamp() + 60,
        aud: DEFAULT_AUDIENCE.to_string(),
        iss: DEFAULT_ISSUER.to_string(),
        ..Default::default()
    };
    let key = EncodingKey::from_secret(state.config.jwt_secret.as_bytes());

    jsonwebtoken::encode(&header, &claims, &key).map_err(|e| {
        tracing::error!("Failed to encode token: {e}");
        InternalError::invalid_argument("Failed to encode token", None)
    })
}
