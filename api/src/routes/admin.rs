use crate::{
    endpoints::{
        common_model, connection_definition, connection_model_definition, connection_model_schema,
        connection_oauth_definition, metrics, openapi,
    },
    server::AppState,
};
use axum::{routing::post, Router};
use std::sync::Arc;
use tower_http::trace::TraceLayer;

pub fn get_router() -> Router<Arc<AppState>> {
    Router::new()
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
        .nest("/common-models", common_model::get_router())
        .layer(TraceLayer::new_for_http())
        .nest("/metrics", metrics::get_router())
}
