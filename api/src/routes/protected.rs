use crate::{
    endpoints::{
        connection, connection_model_definition::test_connection_model_definition, event_access,
        events, metrics, oauth, passthrough, pipeline, transactions, unified,
    },
    middleware::{
        auth,
        blocker::{handle_blocked_error, BlockInvalidHeaders},
        extractor::OwnershipId,
    },
    server::AppState,
};
use axum::{
    error_handling::HandleErrorLayer, middleware::from_fn_with_state, routing::post, Router,
};
use http::HeaderName;
use std::{iter::once, sync::Arc};
use tower::{filter::FilterLayer, ServiceBuilder};
use tower_governor::{governor::GovernorConfigBuilder, GovernorLayer};
use tower_http::{sensitive_headers::SetSensitiveRequestHeadersLayer, trace::TraceLayer};

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
        .nest("/oauth", oauth::get_router())
        .nest("/unified", unified::get_router())
        .layer(TraceLayer::new_for_http())
        .nest("/metrics", metrics::get_router());

    let config = Box::new(
        GovernorConfigBuilder::default()
            .per_second(state.config.burst_rate_limit)
            .burst_size(state.config.burst_size)
            .key_extractor(OwnershipId)
            .use_headers()
            .finish()
            .expect("Failed to build GovernorConfig"),
    );

    routes
        .layer(GovernorLayer {
            config: Box::leak(config),
        })
        .layer(from_fn_with_state(state.clone(), auth::auth))
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
