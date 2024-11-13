use crate::{
    helper::{NamespaceScope, ServiceName},
    server::AppState,
};
use axum::{extract::State, routing::post, Json, Router};
use bson::doc;
use integrationos_domain::{ApplicationError, Connection, Id, IntegrationOSError};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use super::connection::DatabaseConnectionSecret;

pub fn get_router() -> Router<Arc<AppState>> {
    Router::new().route(
        "/mark-as-non-functional",
        post(database_connection_lost_callback),
    )
}

// FIX: This should come from the event, as it should receive the exact same data as the event.
// Move the event to domain and import here
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarkConnectionAsNonFunctionalRequest {
    pub connection_id: Id,
    pub reason: Option<String>,
}

// TODO: Write tests for this endpoint
async fn database_connection_lost_callback(
    // Extension(access): Extension<Arc<EventAccess>>,
    State(state): State<Arc<AppState>>,
    Json(payload): Json<MarkConnectionAsNonFunctionalRequest>,
) -> Result<Json<Connection>, IntegrationOSError> {
    // Instead of direcly updating we're getting the record first so that we can
    // modify the active and deprecated fields from the record metadata
    // without having to update the whole record
    let id = payload.connection_id.to_string();
    let connection = state
        .app_stores
        .connection
        .get_one_by_id(id.as_str())
        .await?;

    match connection {
        None => Err(ApplicationError::not_found(
            &format!("Connection with id {} not found", id),
            None,
        )),
        Some(mut conn) => {
            if conn.record_metadata.active {
                conn.record_metadata.mark_deprecated("system");
                conn.record_metadata.mark_inactive("system");
                conn.record_metadata.mark_updated("system");

                let secret = state
                    .secrets_client
                    .get(&conn.secrets_service_id, &conn.ownership.id)
                    .await?;

                // This means that there's a pod resource that is not running
                // and we need to delete these resources
                if let Ok(secret) = secret.decode::<DatabaseConnectionSecret>() {
                    let namespace: NamespaceScope = secret.namespace.as_str().try_into()?;
                    let service_name = ServiceName::from_id(payload.connection_id)?;

                    tracing::info!(
                        "Deleting all resources for connection {id} in namespace {}",
                        namespace
                    );

                    tracing::info!("service_name: {service_name}");

                    state.k8s_client.delete_all(namespace, service_name).await?;

                    tracing::info!("Deleted all resources for connection {id}");
                }

                let updated = bson::to_document(&conn).map_err(|e| {
                    ApplicationError::bad_request(
                        &format!("Could not serialize connection: {e}"),
                        None,
                    )
                })?;

                state
                    .app_stores
                    .connection
                    .update_one(id.as_str(), doc! { "$set": updated })
                    .await?;
            }

            Ok(Json(conn))
        }
    }
}
