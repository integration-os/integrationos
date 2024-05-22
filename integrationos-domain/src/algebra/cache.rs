use crate::{cache::CacheConfig, IntegrationOSError};
use anyhow::{Context, Result};
use async_trait::async_trait;
use futures::FutureExt;
use redis::{
    aio::{ConnectionLike, ConnectionManager},
    Client, Pipeline, RedisFuture, Value,
};
use serde::{Deserialize, Serialize};
use serde_json::Value as SerdeValue;
use tracing::warn;

#[async_trait]
pub trait CacheExt {
    async fn get_or_insert_with<F>(
        &self,
        key: &str,
        f: F,
        expire: Option<u64>,
    ) -> Result<CacheEntry, IntegrationOSError>
    where
        F: FnOnce() -> Result<CacheEntry, IntegrationOSError> + Send;
    async fn get(&self, key: &str) -> Result<Option<CacheEntry>, IntegrationOSError>;
    async fn set(&self, entry: CacheEntry, expire: Option<u64>) -> Result<(), IntegrationOSError>;
    async fn remove(&self, key: &str) -> Result<(), IntegrationOSError>;
    async fn clear(&self) -> Result<(), IntegrationOSError>;
}

#[derive(Debug, Clone)]
pub struct CacheEntry {
    key: String,
    value: ValueWrapper,
}

impl CacheEntry {
    pub fn new(key: String, value: SerdeValue) -> Self {
        Self {
            key,
            value: ValueWrapper(value),
        }
    }

    pub fn key(&self) -> &str {
        &self.key
    }

    pub fn value(&self) -> &SerdeValue {
        &self.value.0
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ValueWrapper(SerdeValue);

#[derive(Clone)]
pub struct RedisCache {
    client: Client,
    conn: ConnectionManager,
    retry_count: u64,
}

impl RedisCache {
    pub async fn new(config: &CacheConfig, max_retries: u64) -> Result<Self> {
        let client =
            Client::open(config.url.clone()).with_context(|| "Could not parse redis url")?;
        let conn = client
            .get_tokio_connection_manager()
            .await
            .with_context(|| "Could not connect to redis")?;

        Ok(Self {
            client,
            conn,
            retry_count: max_retries,
        })
    }
}

impl ConnectionLike for RedisCache {
    fn get_db(&self) -> i64 {
        self.client.get_connection_info().redis.db
    }

    fn req_packed_command<'a>(&'a mut self, cmd: &'a redis::Cmd) -> RedisFuture<'a, Value> {
        (async move {
            let mut retry_count = 0u64;
            loop {
                let res = self.conn.req_packed_command(cmd).await;
                if res.is_ok() || retry_count >= self.retry_count {
                    return res;
                }
                warn!("Cache failed command, retrying...");
                retry_count += 1;
            }
        })
        .boxed()
    }

    fn req_packed_commands<'a>(
        &'a mut self,
        cmd: &'a Pipeline,
        offset: usize,
        count: usize,
    ) -> RedisFuture<'a, Vec<Value>> {
        (async move {
            let mut retry_count = 0u64;
            loop {
                let res = self.conn.req_packed_commands(cmd, offset, count).await;
                if res.is_ok() || retry_count >= self.retry_count {
                    return res;
                }
                warn!("Cache failed command, retrying...");
                retry_count += 1;
            }
        })
        .boxed()
    }
}
