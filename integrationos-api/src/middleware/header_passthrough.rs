use crate::server::AppState;
use axum::{body::Body, extract::State, middleware::Next, response::Response};
use http::Request;
use integrationos_domain::IntegrationOSError;
use std::sync::Arc;

pub async fn header_passthrough_middleware(
    State(state): State<Arc<AppState>>,
    mut req: Request<Body>,
    next: Next,
) -> Result<Response, IntegrationOSError> {
    let headers = req.headers();
    let include_passthrough = headers
        .get(&state.config.headers.enable_passthrough_header)
        .and_then(|v| v.to_str().ok())
        .map(|s| s == "true")
        .unwrap_or_default();

    req.extensions_mut().insert(Arc::new(include_passthrough));

    Ok(next.run(req).await)
}
