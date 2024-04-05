use std::{iter::once, sync::Arc};

use axum::{
    error_handling::HandleErrorLayer,
    routing::{get, post},
    Router,
};
use http::HeaderName;
use integrationos_domain::common::connection_model_schema::PublicConnectionModelSchema;
use tower::{filter::FilterLayer, ServiceBuilder};
use tower_http::{sensitive_headers::SetSensitiveRequestHeadersLayer, trace::TraceLayer};
use tracing::warn;

use crate::{
    endpoints::{
        connection,
        connection_model_definition::test_connection_model_definition,
        connection_model_schema::{self, public_get_connection_model_schema},
        event_access, events, oauth, passthrough, pipeline, transactions, unified,
    },
    middleware::{
        auth,
        blocker::{handle_blocked_error, BlockInvalidHeaders},
        rate_limiter::{self, RateLimiter},
    },
    server::AppState,
};

pub async fn get_router(state: &Arc<AppState>) -> Router<Arc<AppState>> {
    let r = Router::new()
        .nest("/pipelines", pipeline::get_router())
        .nest("/events", events::get_router())
        .nest("/transactions", transactions::get_router())
        .nest("/connections", connection::get_router())
        .route(
            "/connection-model-definitions/test/:id",
            post(test_connection_model_definition),
        )
        .route(
            "/connection-model-schemas",
            get(public_get_connection_model_schema::<
                connection_model_schema::PublicGetConnectionModelSchema,
                PublicConnectionModelSchema,
            >),
        )
        .nest("/event-access", event_access::get_router())
        .nest("/passthrough", passthrough::get_router())
        .nest("/oauth", oauth::get_router())
        .nest("/unified", unified::get_router());

    let r = match RateLimiter::new(state.clone()).await {
        Ok(rate_limiter) => r.layer(axum::middleware::from_fn_with_state(
            Arc::new(rate_limiter),
            rate_limiter::rate_limiter,
        )),
        Err(e) => {
            warn!("Could not connect to redis: {e}");
            r
        }
    };

    r.layer(axum::middleware::from_fn_with_state(
        state.clone(),
        auth::auth,
    ))
    .layer(TraceLayer::new_for_http())
    .layer(SetSensitiveRequestHeadersLayer::new(once(
        HeaderName::from_lowercase(state.config.headers.auth_header.as_bytes()).unwrap(),
    )))
    .layer(
        ServiceBuilder::new()
            .layer(HandleErrorLayer::new(handle_blocked_error))
            .layer(FilterLayer::new(
                BlockInvalidHeaders::new(state.clone()).await,
            )),
    )
}
