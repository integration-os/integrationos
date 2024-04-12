use super::{get_connection, INTEGRATION_OS_PASSTHROUGH_HEADER};
use crate::{metrics::Metric, server::AppState};
use axum::{
    extract::{Query, State},
    response::IntoResponse,
    routing::get,
    Extension, Router,
};
use http::{header::CONTENT_LENGTH, HeaderMap, HeaderName, Method, Uri};
use hyper::body::Bytes;
use integrationos_domain::{
    common::{
        destination::{Action, Destination},
        event_access::EventAccess,
    },
    ApplicationError, InternalError,
};
use std::{collections::HashMap, sync::Arc};
use tracing::error;

pub fn get_router() -> Router<Arc<AppState>> {
    Router::new().route(
        "/*key",
        get(passthrough_request)
            .post(passthrough_request)
            .patch(passthrough_request)
            .delete(passthrough_request),
    )
}

pub async fn passthrough_request(
    Extension(user_event_access): Extension<Arc<EventAccess>>,
    State(state): State<Arc<AppState>>,
    mut headers: HeaderMap,
    query_params: Option<Query<HashMap<String, String>>>,
    uri: Uri,
    method: Method,
    body: Bytes,
) -> impl IntoResponse {
    let Some(connection_key_header) = headers.get(&state.config.headers.connection_header) else {
        return Err(ApplicationError::bad_request(
            "Connection header not found",
            None,
        ));
    };

    let connection = get_connection(
        user_event_access.as_ref(),
        connection_key_header,
        &state.app_stores,
        &state.connections_cache,
    )
    .await?;

    let destination = Destination {
        platform: connection.platform.clone(),
        action: Action::Passthrough {
            path: uri.path().into(),
            method,
        },
        connection_key: connection.key.clone(),
    };

    let Query(query_params) = query_params.unwrap_or_default();

    headers.remove(&state.config.headers.auth_header);
    headers.remove(&state.config.headers.connection_header);

    let model_execution_result = state
        .extractor_caller
        .send_to_destination(
            Some(connection.clone()),
            &destination,
            headers,
            query_params,
            Some(body.to_vec()),
        )
        .await
        .map_err(|e| {
            error!(
                "Error executing connection model definition in passthrough endpoint: {:?}",
                e
            );

            e
        })?;

    let mut headers = HeaderMap::new();

    model_execution_result
        .headers()
        .into_iter()
        .for_each(|(key, value)| match key {
            &CONTENT_LENGTH => {
                headers.insert(CONTENT_LENGTH, value.clone());
            }
            _ => {
                if let Ok(header_name) =
                    HeaderName::try_from(format!("{INTEGRATION_OS_PASSTHROUGH_HEADER}-{key}"))
                {
                    headers.insert(header_name, value.clone());
                };
            }
        });

    let status = model_execution_result.status();

    let metric = Metric::passthrough(connection);
    if let Err(e) = state.metric_tx.send(metric).await {
        error!("Could not send metric to receiver: {e}");
    }

    let bytes = model_execution_result.bytes().await.map_err(|e| {
        error!(
            "Error retrieving bytes from response in passthrough endpoint: {:?}",
            e
        );

        InternalError::script_error("Error retrieving bytes from response", None)
    })?;

    Ok((status, headers, bytes))
}
