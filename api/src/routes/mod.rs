pub mod admin;
pub mod protected;
pub mod public;

use crate::{
    api_payloads::{ErrorResponse, RootResponse},
    server::AppState,
};
use axum::{response::IntoResponse, routing::get, Json, Router};
use http::StatusCode;
use std::sync::Arc;
use tower_http::cors::CorsLayer;

pub fn get_admin_router(state: &Arc<AppState>) -> Router<Arc<AppState>> {
    let path = format!("/{}", state.config.api_version);
    Router::new()
        .nest(&path, admin::get_router())
        .nest(&path, public::get_router(state))
        .route("/", get(get_root))
        .fallback(not_found_handler)
        .layer(CorsLayer::permissive())
}

pub async fn get_public_router(state: &Arc<AppState>) -> Router<Arc<AppState>> {
    let path = format!("/{}", state.config.api_version);
    let public_path = format!("{path}/public");
    Router::new()
        .nest(&path, protected::get_router(state).await)
        .nest(&public_path, public::get_router(state))
        .route("/", get(get_root))
        .fallback(not_found_handler)
        .layer(CorsLayer::permissive())
}

pub async fn get_root() -> impl IntoResponse {
    Json(RootResponse { success: true })
}

pub async fn not_found_handler() -> impl IntoResponse {
    (
        StatusCode::NOT_FOUND,
        Json(ErrorResponse {
            error: "Not found".to_string(),
        }),
    )
}
