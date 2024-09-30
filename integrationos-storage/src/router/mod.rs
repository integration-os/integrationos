pub mod connection;

use crate::server::AppState;
use axum::{response::IntoResponse, routing::get, Json, Router};
use connection::get_raw;
use http::StatusCode;
use serde_json::json;
use std::sync::Arc;
use tower_http::cors::CorsLayer;

pub async fn get_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/query", get(get_raw))
        .route("/", get(get_root))
        .fallback(not_found_handler)
        .layer(CorsLayer::permissive())
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
