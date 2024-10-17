use super::storage::Storage;
use crate::{
    domain::postgres::PostgresDatabaseConnection,
    server::{AppState, Server},
};
use axum::async_trait;
use integrationos_domain::database::DatabaseConnectionConfig;
use std::sync::Arc;

#[async_trait]
pub trait Initializer {
    async fn init(config: &DatabaseConnectionConfig) -> Result<Server, anyhow::Error>;
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
}
