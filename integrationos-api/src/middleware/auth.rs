use crate::{endpoints::ApiError, internal_server_error, server::AppState, unauthorized};
use axum::{body::Body, extract::State, middleware::Next, response::Response};
use http::Request;
use integrationos_domain::{ApplicationError, InternalError};
use mongodb::bson::doc;
use std::sync::Arc;
use tracing::error;

pub async fn auth(
    State(state): State<Arc<AppState>>,
    mut req: Request<Body>,
    next: Next,
) -> Result<Response, ApiError> {
    let Some(auth_header) = req.headers().get(&state.config.headers.auth_header) else {
        return Err(unauthorized!());
    };

    if let Some(conn_header) = req.headers().get(&state.config.headers.connection_header) {
        // environment can be live or test
        // connection header value starts with environment
        // auth header value starts with either id_ or sk_ and then the environment
        // Make sure the environments match, or we return 404
        if conn_header.as_bytes()[..4] != auth_header.as_bytes()[3..7] {
            return Err(unauthorized!());
        }
    }

    let event_access_result = state
        .cache
        .try_get_with_by_ref(auth_header, async {
            let key = auth_header
                .to_str()
                // A bad header value is a user error, so we return not found
                .map_err(|_| ApplicationError::not_found("Invalid auth header", None))?;

            let event_access = state
                .app_stores
                .event_access
                .get_one(doc! {
                    "accessKey": key,
                    "deleted": false
                })
                .await
                .map_err(|e| InternalError::connection_error(e.as_ref(), None))?;

            if let Some(event_access) = event_access {
                Ok(Arc::new(event_access))
            } else {
                Err(ApplicationError::not_found("Event access", None))
            }
        })
        .await;

    match event_access_result {
        Ok(data) => {
            req.extensions_mut().insert(data);
            Ok(next.run(req).await)
        }
        Err(e) => {
            if e.is_application() {
                Err(unauthorized!())
            } else {
                error!("Error fetching auth data: {:?}", e);

                Err(internal_server_error!())
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
