use super::{on_error_callback, storage::Storage};
use crate::{
    domain::postgres::PostgresDatabaseConnection,
    server::{AppState, Server},
};
use axum::async_trait;
use http::header::AUTHORIZATION;
use integrationos_domain::{
    database::{DatabaseConnectionType, DatabasePodConfig},
    database_secret::DatabaseConnectionSecret,
    emitted_events::DatabaseConnectionLost,
    Claims, Id, InternalError, Secret, Unit,
};
use reqwest::Client;
use std::{str::FromStr, sync::Arc};

#[async_trait]
pub trait Initializer {
    async fn init(config: &DatabasePodConfig) -> Result<Server, anyhow::Error>;
    async fn kill(
        config: &DatabasePodConfig,
        reason: String,
        client: Option<Client>,
    ) -> Result<Unit, anyhow::Error>;
}

#[async_trait]
impl Initializer for PostgresDatabaseConnection {
    async fn init(config: &DatabasePodConfig) -> Result<Server, anyhow::Error> {
        let jwt_secret = match config.jwt_secret.clone() {
            Some(jwt_secret) => jwt_secret,
            None => {
                let error = "JWT secret is required for database connection";

                tracing::error!("{error}");
                on_error_callback(&anyhow::anyhow!("{error}"), config, None).await?;
                return Err(anyhow::anyhow!(error));
            }
        };

        match config.database_connection_type {
            DatabaseConnectionType::PostgreSql => {
                let client = Client::new();

                let uri = format!(
                    "{}/v1/admin/connection/{}",
                    config.connections_url, config.connection_id
                );

                let authorization = Claims::from_secret(jwt_secret.as_str())?;
                let secret = client
                    .get(uri)
                    .header(AUTHORIZATION, format!("Bearer {authorization}"))
                    .send()
                    .await
                    .map_err(|e| {
                        InternalError::io_err(&format!("Failed to get secret: {e}"), None)
                    });

                let secret = match secret {
                    Ok(secret) => secret.json::<Secret>().await.map_err(|e| {
                        InternalError::deserialize_error(
                            &format!("Failed to deserialize secret: {e}"),
                            None,
                        )
                    })?,
                    Err(e) => {
                        on_error_callback(&e.clone().into(), config, Some(client)).await?;
                        return Err(e.into());
                    }
                };

                let postgres =
                    PostgresDatabaseConnection::new(&secret.decode::<DatabaseConnectionSecret>()?)
                        .await;

                match postgres {
                    Ok(postgres) => {
                        let storage: Arc<dyn Storage> = Arc::new(postgres);

                        Ok(Server {
                            state: Arc::new(AppState {
                                config: config.clone(),
                                storage,
                            }),
                        })
                    }
                    Err(e) => {
                        on_error_callback(&e, config, Some(client)).await?;
                        Err(e)
                    }
                }
            }
        }
    }

    async fn kill(
        config: &DatabasePodConfig,
        reason: String,
        client: Option<Client>,
    ) -> Result<Unit, anyhow::Error> {
        let emit_url = config.emit_url.clone();
        let client = client.unwrap_or_default();
        let connection_id = Id::from_str(&config.connection_id)?;
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
            .json(&value)
            .send()
            .await?;

        tracing::info!("Event for dispose of connection {connection_id} emitted");

        Ok(())
    }
}
