use http::header::AUTHORIZATION;
use integrationos_domain::{
    database::DatabasePodConfig, emitted_events::ConnectionLostReason, ApplicationError, Claims,
    Id, IntegrationOSError, InternalError, Unit,
};
use reqwest::Client;
use std::str::FromStr;

pub mod init;
pub mod storage;

pub async fn on_error_callback(
    e: &anyhow::Error,
    config: &DatabasePodConfig,
    client: Option<Client>,
) -> Result<Unit, IntegrationOSError> {
    let base_path = &config.connections_url;
    let connection_id = Id::from_str(&config.connection_id)?;
    let jwt_secret = config
        .jwt_secret
        .clone()
        .ok_or(ApplicationError::bad_request(
            "JWT secret is required for database connection",
            None,
        ))?;

    let path = format!("{base_path}/v1/database-connection-lost/{connection_id}");

    let authorization = Claims::from_secret(jwt_secret.as_str())?;
    let payload = ConnectionLostReason {
        reason: e.to_string(),
    };
    let client = client.unwrap_or_default();

    client
        .post(path)
        .json(&payload)
        .header(AUTHORIZATION, format!("Bearer {authorization}"))
        .send()
        .await
        .inspect(|res| {
            tracing::info!("Response: {:?}", res);
        })
        .map_err(|e| {
            tracing::error!("Failed to build request for connection id {connection_id}: {e}");
            InternalError::io_err(
                &format!("Failed to build request for connection id {connection_id}"),
                None,
            )
        })?
        .error_for_status()
        .map_err(|e| {
            tracing::error!("Failed to execute request for connection id {connection_id}: {e}");
            ApplicationError::bad_request(
                &format!("Failed to execute request for connection id {connection_id}"),
                None,
            )
        })
        .map(|res| tracing::info!("Response: {:?}", res))?;

    Ok(())
}
