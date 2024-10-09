use super::storage::Storage;
use crate::{
    domain::{config::DatabaseConnectionConfig, postgres::PostgresStorage},
    server::{AppState, Server},
};
use axum::async_trait;
use std::sync::Arc;

#[async_trait]
pub trait Initializer {
    async fn init(config: &DatabaseConnectionConfig) -> Result<Server, anyhow::Error>;
}

#[async_trait]
impl Initializer for PostgresStorage {
    async fn init(config: &DatabaseConnectionConfig) -> Result<Server, anyhow::Error> {
        let postgres: PostgresStorage = PostgresStorage::new(config).await?;
        let storage: Arc<dyn Storage> = Arc::new(postgres);

        Ok(Server {
            state: Arc::new(AppState {
                config: config.clone(),
                storage,
            }),
        })
    }
}
