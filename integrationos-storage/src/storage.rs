use std::sync::{Arc, Mutex};
use std::time::Duration;
use anyhow::Result as AnyhowResult;
use async_trait::async_trait;
use futures::stream::BoxStream;
use futures::{StreamExt, TryStreamExt};
use integrationos_domain::{ApplicationError, IntegrationOSError};
use sqlx::postgres::{PgConnectOptions, PgPoolOptions, PgRow, PgSslMode};
use sqlx::{query, PgPool};
use crate::config::StorageConfig;

const INITIAL_LIMIT: usize = 100;
type PgResult = BoxStream<'static, Result<PgRow, IntegrationOSError>>;
pub type ThreadSafePgResult = Arc<Mutex<PgResult>>;


#[async_trait]
pub trait Storage: Send + Sync {
    type Result;

    async fn execute_raw(&self, query: &'static str) -> Result<Self::Result, IntegrationOSError>;
}

#[derive(Clone)]
pub struct PostgresStorage {
    pub pool: PgPool,
}

impl PostgresStorage {
    pub async fn new(configuration: &StorageConfig) -> AnyhowResult<Self> {
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

#[async_trait]
impl Storage for PostgresStorage {
    type Result = ThreadSafePgResult;

    async fn execute_raw(&self, sql: &'static str) -> Result<Self::Result, IntegrationOSError> {
        let stream = query(sql)
            .fetch(&self.pool)
            .take(INITIAL_LIMIT)
            .map_err(|e| {
                ApplicationError::bad_request(&format!("Failed to execute query: {}", e), None)
            })
            .boxed();
        Ok(Arc::new(Mutex::new(stream)))
    }
}
