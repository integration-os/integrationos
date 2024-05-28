use crate::{
    endpoints::{
        common_enum, common_model, connection_definition, connection_model_schema,
        connection_oauth_definition, event_access::create_event_access_for_new_user, openapi, read,
    },
    middleware::jwt_auth::{self, JwtState},
    server::AppState,
};
use axum::{
    middleware::from_fn_with_state,
    routing::{get, post},
    Router,
};
use integrationos_domain::{
    common_model::CommonModel, connection_definition::ConnectionDefinition,
};
use std::sync::Arc;
use tower_http::trace::TraceLayer;

pub fn get_router(state: &Arc<AppState>) -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/event-access/default",
            post(create_event_access_for_new_user).layer(from_fn_with_state(
                Arc::new(JwtState::new(state)),
                jwt_auth::jwt_auth,
            )),
        )
        .route(
            "/connection-definitions",
            get(read::<connection_definition::CreateRequest, ConnectionDefinition>),
        )
        .route(
            "/connection-oauth-definition-schema",
            get(read::<
                connection_oauth_definition::FrontendOauthConnectionDefinition,
                connection_oauth_definition::FrontendOauthConnectionDefinition,
            >),
        )
        .route("/openapi", get(openapi::get_openapi))
        .route("/openapi/yaml", get(openapi::get_openapi_yaml))
        .route(
            "/connection-data/models/:platform_name",
            get(connection_model_schema::get_platform_models),
        )
        .nest(
            "/sdk",
            Router::new()
                .route(
                    "/common-models",
                    get(read::<common_model::CreateRequest, CommonModel>),
                )
                .route("/common-enums", get(common_enum::read)),
        )
        .route(
            "/connection-data/:model/:platform_name",
            get(connection_definition::public_get_connection_details),
        )
        .layer(TraceLayer::new_for_http())
}
