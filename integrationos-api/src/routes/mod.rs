pub mod authenticated;
pub mod protected;
pub mod public;

use crate::{
    api_payloads::{ErrorResponse, RootResponse},
    server::AppState,
};
use axum::{
    body::Body, extract::Request, middleware::Next, response::IntoResponse, routing::get, Json,
    Router,
};
use http::StatusCode;
use integrationos_domain::TimedExt;
use std::sync::Arc;
use tower_http::cors::CorsLayer;

pub async fn get_router(state: &Arc<AppState>) -> Router<Arc<AppState>> {
    let path = format!("/{}", state.config.api_version);
    let public_path = format!("{path}/public");
    Router::new()
        .nest(&public_path, public::get_router(state))
        .nest(&path, protected::get_router(state).await)
        .nest(&path, authenticated::get_router(state).await)
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

pub async fn log_request_middleware(
    req: Request<Body>,
    next: Next,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let path = req.uri().path().to_string();
    let method = req.method().to_string();
    let res = next
        .run(req)
        .timed(|response, elapsed| {
            let status = response.status();
            let logger = |str| {
                if status.is_server_error() {
                    tracing::error!("{}", str)
                } else if status.is_client_error() {
                    tracing::warn!("{}", str)
                } else {
                    tracing::info!("{}", str)
                }
            };

            logger(format!(
                "[{} {}] Elapsed time: {}ms | Status: {}",
                method,
                path,
                elapsed.as_millis(),
                status,
            ));
        })
        .await;

    Ok(res)
}
