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
    Claims, InternalError, Secret,
};
use reqwest::Client;
use std::sync::Arc;

#[async_trait]
pub trait Initializer {
    async fn init(config: &DatabasePodConfig) -> Result<Server, anyhow::Error>;
}

pub struct DatabaseInitializer;

#[async_trait]
impl Initializer for DatabaseInitializer {
    async fn init(config: &DatabasePodConfig) -> Result<Server, anyhow::Error> {
        let server = start(config).await;

        if let Err(e) = server {
            on_error_callback(&e, config, None).await?;
            return Err(e);
        }

        server
    }
}

async fn start(config: &DatabasePodConfig) -> Result<Server, anyhow::Error> {
    let jwt_secret = match config.jwt_secret.clone() {
        Some(jwt_secret) => jwt_secret,
        None => {
            let error = "JWT secret is required for database connection";

            tracing::error!("{error}");
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
                .map_err(|e| InternalError::io_err(&format!("Failed to get secret: {e}"), None));

            let secret = match secret {
                Ok(secret) => secret.json::<Secret>().await.map_err(|e| {
                    InternalError::deserialize_error(
                        &format!("Failed to deserialize secret: {e}"),
                        None,
                    )
                })?,
                Err(e) => {
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
                Err(e) => Err(e),
            }
        }
    }
}
