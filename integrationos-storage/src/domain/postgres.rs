use std::time::Duration;
use super::config::StorageConfig;
use anyhow::Result;
use sqlx::{
    postgres::{PgConnectOptions, PgPoolOptions, PgSslMode},
    PgPool,
};

#[derive(Clone)]
pub struct PostgresStorage {
    pub pool: PgPool,
}

impl PostgresStorage {
    pub async fn new(configuration: &StorageConfig) -> Result<Self> {
        let options = PgConnectOptions::new()
            .username(&configuration.postgres_config.user)
            .password(&configuration.postgres_config.password)
            .host(&configuration.postgres_config.host)
            .ssl_mode(if configuration.postgres_config.ssl {
                PgSslMode::Require
            } else {
                PgSslMode::Disable
            })
            .port(configuration.postgres_config.port);

        let pool = PgPoolOptions::new()
            .max_connections(configuration.postgres_config.pool_size)
            .acquire_timeout(Duration::from_millis(configuration.postgres_config.timeout))
            .connect_with(options.database(&configuration.postgres_config.name))
            .await?;

        Ok(Self { pool })
    }
}
