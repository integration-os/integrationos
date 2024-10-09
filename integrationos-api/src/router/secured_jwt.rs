use crate::{
    logic::{
        common_enum, common_model, connection_definition,
        connection_model_definition::{self},
        connection_model_schema, connection_oauth_definition, openapi, platform, platform_page,
    },
    middleware::jwt_auth::{self, JwtState},
    server::AppState,
};
use axum::{
    middleware::{from_fn, from_fn_with_state},
    routing::post,
    Router,
};
use std::sync::Arc;
use tower_http::trace::TraceLayer;

use super::log_request_middleware;

pub async fn get_router(state: &Arc<AppState>) -> Router<Arc<AppState>> {
    let routes = Router::new()
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
        .nest("/platforms", platform::get_router())
        .nest("/platform-pages", platform_page::get_router())
        .nest("/common-models", common_model::get_router())
        .nest("/common-enums", common_enum::get_router());

    routes
        .layer(from_fn_with_state(
            Arc::new(JwtState::new(state)),
            jwt_auth::jwt_auth,
        ))
        .layer(from_fn(log_request_middleware))
        .layer(TraceLayer::new_for_http())
}
