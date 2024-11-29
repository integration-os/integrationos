use super::storage::Storage;
use crate::{
    domain::postgres::PostgresDatabaseConnection,
    server::{AppState, Server},
};
use axum::async_trait;
use integrationos_domain::{database::DatabaseConnectionConfig, Unit};
use reqwest::Client;
use serde_json::json;
use std::sync::Arc;

#[async_trait]
pub trait Initializer {
    async fn init(config: &DatabaseConnectionConfig) -> Result<Server, anyhow::Error>;
    async fn kill(config: &DatabaseConnectionConfig) -> Result<Unit, anyhow::Error>;
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

    async fn kill(config: &DatabaseConnectionConfig) -> Result<Unit, anyhow::Error> {
        let emit_url = config.emit_url.clone();
        let connection_id = config.connection_id.clone();
        let client = Client::new();
        let value = json!({
            "type": "DatabaseConnectionLost",
            "connectionId": connection_id
        });

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
