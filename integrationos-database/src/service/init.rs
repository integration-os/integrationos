use super::storage::Storage;
use crate::{
    domain::postgres::PostgresDatabaseConnection,
    server::{AppState, Server},
};
use axum::async_trait;
use integrationos_domain::{
    database::DatabaseConnectionConfig, emitted_events::DatabaseConnectionLost, Id, Unit,
};
use reqwest::Client;
use std::{str::FromStr, sync::Arc};

#[async_trait]
pub trait Initializer {
    async fn init(config: &DatabaseConnectionConfig) -> Result<Server, anyhow::Error>;
    async fn kill(config: &DatabaseConnectionConfig, reason: String)
        -> Result<Unit, anyhow::Error>;
}

#[async_trait]
impl Initializer for PostgresDatabaseConnection {
    async fn init(config: &DatabaseConnectionConfig) -> Result<Server, anyhow::Error> {
        let postgres: PostgresDatabaseConnection = PostgresDatabaseConnection::new(config).await?;
        let storage: Arc<dyn Storage> = Arc::new(postgres);

        Ok(Server {
            state: Arc::new(AppState {
                config: config.clone(),
                storage,
            }),
        })
    }

    async fn kill(
        config: &DatabaseConnectionConfig,
        reason: String,
    ) -> Result<Unit, anyhow::Error> {
        let emit_url = config.emit_url.clone();
        let connection_id = Id::from_str(&config.connection_id)?;
        let client = Client::new();
        let value = DatabaseConnectionLost {
            connection_id,
            reason: Some(reason),
            schedule_on: None,
        }
        .as_event();

        tracing::info!("Emitting event {value:?} to dispose of connection {connection_id}");

        client
            .post(format!("{}/v1/emit", emit_url))
            .header("content-type", "application/json")
            .body(value.to_string())
            .send()
            .await?;

        Ok(())
    }
}
