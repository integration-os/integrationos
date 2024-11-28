use crate::server::AppState;
use axum::{body::Body, extract::State, middleware::Next, response::Response};
use http::Request;
use integrationos_domain::{ApplicationError, IntegrationOSError, InternalError};
use mongodb::bson::doc;
use std::sync::Arc;
use tracing::error;

pub async fn header_auth_middleware(
    State(state): State<Arc<AppState>>,
    mut req: Request<Body>,
    next: Next,
) -> Result<Response, IntegrationOSError> {
    let Some(auth_header) = req.headers().get(&state.config.headers.auth_header) else {
        return Err(ApplicationError::unauthorized(
            "You're not authorized to access this resource",
            None,
        ));
    };

    if let Some(conn_header) = req.headers().get(&state.config.headers.connection_header) {
        // environment can be live or test
        // connection header value starts with environment
        // auth header value starts with either id_ or sk_ and then the environment
        // Make sure the environments match, or we return 404
        if conn_header.as_bytes()[..4] != auth_header.as_bytes()[3..7] {
            return Err(ApplicationError::not_found(
                "Invalid connection header",
                None,
            ));
        }
    }

    let key = auth_header
        .to_str()
        .map_err(|_| ApplicationError::not_found("Invalid auth header", None))?;

    let event_access_result = state
        .event_access_cache
        .get_or_insert_with_filter(
            auth_header,
            state.app_stores.event_access.clone(),
            doc! {
                "accessKey": key,
                "deleted": false
            },
        )
        .await;

    match event_access_result {
        Ok(data) => {
            req.extensions_mut().insert(Arc::new(data));
            Ok(next.run(req).await)
        }
        Err(e) => {
            if e.is_application() {
                Err(e)
            } else {
                error!("Error fetching auth data: {:?}", e);

                Err(InternalError::unknown("Error fetching auth data", None))
            }
        }
    }
}

#[cfg(test)]
mod test {
    #[test]
    fn test_header_check() {
        let conn = b"test::key";
        let access_key = b"id_test_foo";
        assert_eq!(conn[..4], access_key[3..7]);
    }
}
