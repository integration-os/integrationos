use crate::{logic::emitter, server::AppState};
use axum::{middleware::from_fn, response::IntoResponse, routing::get, Json, Router};
use http::StatusCode;
use integrationos_domain::telemetry::log_request_middleware;
use serde_json::json;
use std::sync::Arc;
use tower_http::{cors::CorsLayer, trace::TraceLayer};

pub async fn get_router(state: &Arc<AppState>) -> Router<Arc<AppState>> {
    let path = format!("/{}", state.config.api_version);
    let metrics_layer = state.metrics.as_ref().0.clone();
    Router::new()
        .nest(&path, emitter::get_router())
        .route("/", get(get_root))
        .fallback(not_found_handler)
        .layer(CorsLayer::permissive())
        .layer(from_fn(log_request_middleware))
        .layer(metrics_layer)
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
