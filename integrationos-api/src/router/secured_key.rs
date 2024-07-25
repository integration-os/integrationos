use super::log_request_middleware;
use crate::{
    logic::{
        connection,
        connection_model_definition::test_connection_model_definition,
        connection_model_schema::{
            public_get_connection_model_schema, PublicGetConnectionModelSchema,
        },
        event_access, events, metrics, oauth, passthrough, pipeline, transactions, unified,
    },
    middleware::{
        blocker::{handle_blocked_error, BlockInvalidHeaders},
        extractor::{rate_limit, RateLimiter},
        header_auth,
    },
    server::AppState,
};
use axum::{
    error_handling::HandleErrorLayer,
    middleware::{from_fn, from_fn_with_state},
    routing::{get, post},
    Router,
};
use http::HeaderName;
use integrationos_domain::connection_model_schema::PublicConnectionModelSchema;
use std::{iter::once, sync::Arc};
use tower::{filter::FilterLayer, ServiceBuilder};
use tower_http::{sensitive_headers::SetSensitiveRequestHeadersLayer, trace::TraceLayer};
use tracing::warn;

pub async fn get_router(state: &Arc<AppState>) -> Router<Arc<AppState>> {
    let routes = Router::new()
        .nest("/pipelines", pipeline::get_router())
        .nest("/events", events::get_router())
        .nest("/transactions", transactions::get_router())
        .nest("/connections", connection::get_router())
        .nest("/event-access", event_access::get_router())
        .nest("/passthrough", passthrough::get_router())
        .route(
            "/connection-model-definitions/test/:id",
            post(test_connection_model_definition),
        )
        .route(
            "/connection-model-schema",
            get(public_get_connection_model_schema::<
                PublicGetConnectionModelSchema,
                PublicConnectionModelSchema,
            >),
        )
        .nest("/oauth", oauth::get_router())
        .nest("/unified", unified::get_router())
        .layer(TraceLayer::new_for_http())
        .nest("/metrics", metrics::get_router());

    let routes = match RateLimiter::new(state.clone()).await {
        Ok(rate_limiter) => routes.layer(axum::middleware::from_fn_with_state(
            Arc::new(rate_limiter),
            rate_limit,
        )),
        Err(e) => {
            warn!("Could not connect to redis: {e}");
            routes
        }
    };

    routes
        .layer(from_fn_with_state(state.clone(), header_auth::header_auth))
        .layer(from_fn(log_request_middleware))
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
