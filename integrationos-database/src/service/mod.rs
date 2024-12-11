use crate::domain::postgres::PostgresDatabaseConnection;
use init::Initializer;
use integrationos_domain::{database::DatabasePodConfig, Unit};
use reqwest::Client;

pub mod init;
pub mod storage;

pub async fn on_error_callback(
    e: &anyhow::Error,
    config: &DatabasePodConfig,
    client: Option<Client>,
) -> Result<Unit, anyhow::Error> {
    if config.emitter_enabled {
        PostgresDatabaseConnection::kill(config, e.to_string(), client).await
    } else {
        Ok(())
    }
}
