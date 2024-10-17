use integrationos_domain::{cache::CacheConfig, IntegrationOSError, InternalError};
use redis::aio::{ConnectionManager, ConnectionManagerConfig};
use std::time::Duration;

pub struct RedisCache {
    pub inner: ConnectionManager,
}

impl RedisCache {
    pub async fn new(configuration: &CacheConfig) -> Result<Self, IntegrationOSError> {
        let client = redis::Client::open(configuration.url.clone()).map_err(|e| {
            tracing::warn!("Error creating the pool: {:?}", e);
            InternalError::io_err("There was an error with the configuration", None)
        })?;

        let config = ConnectionManagerConfig::new()
            .set_max_delay(configuration.max_delay)
            .set_response_timeout(Duration::from_secs(configuration.response_timeout))
            .set_connection_timeout(Duration::from_secs(configuration.connection_timeout));

        let inner = ConnectionManager::new_with_config(client, config)
            .await
            .map_err(|e| {
                tracing::warn!("Error creating the pool: {:?}", e);
                InternalError::io_err("There was an error with the configuration", None)
            })?;

        Ok(Self { inner })
    }
}
