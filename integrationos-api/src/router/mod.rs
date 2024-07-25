pub mod public;
pub mod secured_jwt;
pub mod secured_key;

use crate::server::AppState;
use axum::{
    body::Body, extract::Request, middleware::Next, response::IntoResponse, routing::get, Json,
    Router,
};
use http::StatusCode;
use integrationos_domain::TimedExt;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;
use tower_http::cors::CorsLayer;

#[derive(Serialize, Deserialize, Debug)]
pub struct ServerResponse<T>
where
    T: Serialize,
{
    #[serde(rename = "type")]
    pub response_type: String,
    #[serde(flatten)]
    pub args: T,
}

impl<T> ServerResponse<T>
where
    T: Serialize,
{
    pub fn new<R>(response_type: R, args: T) -> Self
    where
        R: Into<String>,
    {
        Self {
            response_type: response_type.into(),
            args,
        }
    }

    pub fn error(error: T) -> Self {
        Self {
            response_type: "error".to_string(),
            args: error,
        }
    }
}

pub async fn get_router(state: &Arc<AppState>) -> Router<Arc<AppState>> {
    let path = format!("/{}", state.config.api_version);
    let public_path = format!("{path}/public");
    Router::new()
        .nest(&public_path, public::get_router(state))
        .nest(&path, secured_key::get_router(state).await)
        .nest(&path, secured_jwt::get_router(state).await)
        .route("/", get(get_root))
        .fallback(not_found_handler)
        .layer(CorsLayer::permissive())
}

pub async fn get_root() -> impl IntoResponse {
    Json(ServerResponse {
        response_type: "success".to_string(),
        args: json!({
            "success": true,
        }),
    })
}

pub async fn not_found_handler() -> impl IntoResponse {
    (
        StatusCode::NOT_FOUND,
        Json(ServerResponse {
            response_type: "error".to_string(),
            args: json!({
                "error": "Not found", // For backward compatibility
            }),
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
