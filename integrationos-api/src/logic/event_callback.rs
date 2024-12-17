use crate::{helper::ServiceName, server::AppState};
use axum::{
    extract::{Path, State},
    routing::post,
    Json, Router,
};
use bson::doc;
use integrationos_domain::{
    database_secret::DatabaseConnectionSecret, emitted_events::ConnectionLostReason,
    ApplicationError, Connection, Id, IntegrationOSError,
};
use std::sync::Arc;

pub fn get_router() -> Router<Arc<AppState>> {
    Router::new().route(
        "/database-connection-lost/:connection_id",
        post(database_connection_lost_callback),
    )
}

async fn database_connection_lost_callback(
    State(state): State<Arc<AppState>>,
    Path(connection_id): Path<Id>,
    Json(reason): Json<ConnectionLostReason>,
) -> Result<Json<Connection>, IntegrationOSError> {
    // Instead of direcly updating we're getting the record first so that we can
    // modify the active and deprecated fields from the record metadata
    // without having to update the whole record
    let id = connection_id.to_string();
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
                conn.mark_error(reason.reason.as_str());
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
                    let service_name = ServiceName::from_id(connection_id)?;
                    let namespace = secret.namespace;

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
