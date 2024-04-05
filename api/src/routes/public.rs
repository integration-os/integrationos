use std::sync::Arc;

use axum::{
    routing::{get, post},
    Router,
};
use integrationos_domain::{
    common_model::CommonModel, connection_definition::ConnectionDefinition,
};
use tower_http::trace::TraceLayer;

use crate::{
    endpoints::{
        common_enum, common_model, connection_definition, connection_model_schema,
        connection_oauth_definition, event_access::create_event_access_for_new_user, openapi, read,
        read_cached,
    },
    middleware::jwt_auth::{self, JwtState},
    server::AppState,
};

const OBFUSCATED_ENDPOINT: &str =
    "/e7262bf18c81bc1ff7f726e6d1a6da59f6e77dde0d63d9b60c041af57be8c197";

pub fn get_router(state: &Arc<AppState>) -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/event-access/default",
            post(create_event_access_for_new_user).layer(axum::middleware::from_fn_with_state(
                Arc::new(JwtState::new(state)),
                jwt_auth::jwt_auth,
            )),
        )
        .route(
            "/connection-definitions",
            get(read_cached::<connection_definition::CreateRequest, ConnectionDefinition>),
        )
        .route(
            "/connection-oauth-definition-schema",
            get(read_cached::<
                connection_oauth_definition::FrontendOauthConnectionDefinition,
                connection_oauth_definition::FrontendOauthConnectionDefinition,
            >),
        )
        .route("/openapi", get(openapi::get_openapi))
        .route(
            "/connection-data/models/:platform_name",
            get(connection_model_schema::public_get_platform_models),
        )
        .route(
            "/connection-data/:model/:platform_name",
            get(connection_definition::public_get_connection_details),
        )
        .nest(
            OBFUSCATED_ENDPOINT,
            Router::new()
                .route(
                    "/common-models",
                    get(read::<common_model::CreateRequest, CommonModel>),
                )
                .route("/common-enums", get(common_enum::read)),
        )
        .layer(TraceLayer::new_for_http())
}
