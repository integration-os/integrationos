use crate::{
    endpoints::{
        common_model, connection_definition,
        connection_model_definition::{self, test_connection_model_definition},
        connection_model_schema, connection_oauth_definition, openapi,
    },
    middleware::{
        extractor::OwnershipId,
        jwt_auth::{self, JwtState},
    },
    server::AppState,
};
use axum::{middleware::from_fn_with_state, routing::post, Router};
use std::sync::Arc;
use tower_governor::{governor::GovernorConfigBuilder, GovernorLayer};
use tower_http::trace::TraceLayer;

pub async fn get_router(state: &Arc<AppState>) -> Router<Arc<AppState>> {
    let routes = Router::new()
        .route(
            "/connection-model-definitions/test/:id",
            post(test_connection_model_definition),
        )
        .nest(
            "/connection-definitions",
            connection_definition::get_router(),
        )
        .nest(
            "/connection-oauth-definitions",
            connection_oauth_definition::get_router(),
        )
        .nest(
            "/connection-model-definitions",
            connection_model_definition::get_router(),
        )
        .route("/openapi", post(openapi::refresh_openapi))
        .nest(
            "/connection-model-schemas",
            connection_model_schema::get_router(),
        )
        .nest("/common-models", common_model::get_router());

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
        .layer(from_fn_with_state(
            Arc::new(JwtState::new(state)),
            jwt_auth::jwt_auth,
        ))
        .layer(TraceLayer::new_for_http())
}
