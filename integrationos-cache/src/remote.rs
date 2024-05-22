use crate::RemoteCacheExt;
use deadpool_redis::{
    redis::{cmd, AsyncCommands, RedisError},
    Connection, Pool, PoolConfig, Runtime, Timeouts,
};
use futures::Future;
use integrationos_domain::{cache::CacheConfig, IntegrationOSError, InternalError, Unit};
use redis::{from_redis_value, FromRedisValue, RedisResult, RedisWrite, ToRedisArgs};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::Duration;

pub struct RedisCache {
    pool: Pool,
    // Make the inner connection public for backward compatibility
    pub inner: Connection,
}

#[derive(Debug, Clone)]
struct CacheEntry {
    key: String,
    value: Option<ValueWrapper>,
}

impl CacheEntry {
    pub fn new(key: String, value: Value) -> Self {
        Self {
            key,
            value: Some(ValueWrapper(value)),
        }
    }

    fn get_as<T>(&self) -> Option<T>
    where
        T: for<'de> Deserialize<'de>,
    {
        match &self.value {
            Some(value) => serde_json::from_value(value.0.clone()).ok(),
            None => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ValueWrapper(Value);

impl ToRedisArgs for ValueWrapper {
    fn write_redis_args<W>(&self, out: &mut W)
    where
        W: ?Sized + RedisWrite,
    {
        let json_str = serde_json::to_string(&self).unwrap();

        out.write_arg(json_str.as_bytes());
    }
}

impl FromRedisValue for ValueWrapper {
    fn from_redis_value(v: &deadpool_redis::redis::Value) -> RedisResult<Self> {
        let json_str: Option<String> = from_redis_value(v)?;

        match json_str {
            Some(json_str) => {
                let json: Value = serde_json::from_str(json_str.as_str()).map_err(|error| {
                    RedisError::from(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        error.to_string(),
                    ))
                })?;

                Ok(Self(json))
            }
            None => Ok(Self(Value::Null)),
        }
    }
}

impl ToRedisArgs for CacheEntry {
    fn write_redis_args<W>(&self, out: &mut W)
    where
        W: ?Sized + RedisWrite,
    {
        let json = serde_json::json!(
            {
                "key": self.key,
                "value": self.value
            }
        );

        let json_str = json.to_string();

        out.write_arg(json_str.as_bytes());
    }
}

impl FromRedisValue for CacheEntry {
    fn from_redis_value(v: &deadpool_redis::redis::Value) -> RedisResult<Self> {
        let json_str: Option<String> = from_redis_value(v)?;

        match json_str {
            Some(json_str) => {
                let json: Value = serde_json::from_str(json_str.as_str()).map_err(|error| {
                    RedisError::from(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        error.to_string(),
                    ))
                })?;

                let key = json.get("key").ok_or(RedisError::from(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "key not found",
                )))?;
                let key = key.as_str().ok_or(RedisError::from(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "key not found",
                )))?;
                let key = key.to_string();

                let value = json
                    .get("value")
                    .ok_or(RedisError::from(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "value not found",
                    )))?;
                let value = value.clone();

                Ok(Self {
                    key,
                    value: Some(ValueWrapper(value)),
                })
            }
            None => Ok(Self {
                key: Default::default(),
                value: None,
            }),
        }
    }
}

impl RedisCache {
    pub async fn new(configuration: &CacheConfig) -> Result<Self, IntegrationOSError> {
        let redis = deadpool_redis::Config {
            url: Some(configuration.url.clone()),
            connection: None,
            pool: Some(PoolConfig {
                max_size: configuration.pool_size,
                timeouts: Timeouts {
                    wait: Some(Duration::from_secs(configuration.wait_timeout)),
                    create: Some(Duration::from_secs(configuration.create_timeout)),
                    recycle: Some(Duration::from_secs(configuration.recycle_timeout)),
                },
                ..Default::default()
            }),
        };
        let pool = redis.create_pool(Some(Runtime::Tokio1)).map_err(|warn| {
            tracing::warn!("Error creating the pool: {:?}", warn);
            InternalError::io_err("There was an warn with the configuration", None)
        })?;

        let inner = pool.get().await.map_err(|warn| {
            tracing::warn!("Error getting the connection: {:?}", warn);
            InternalError::io_err("There was an warn with the connection", None)
        })?;

        Ok(Self { pool, inner })
    }

    pub async fn connection(&self) -> Result<Connection, IntegrationOSError> {
        let connection = self.pool.get().await.map_err(|warn| {
            tracing::warn!("Error getting the connection: {:?}", warn);
            InternalError::io_err("There was an warn with the connection", None)
        })?;

        Ok(connection)
    }

    #[tracing::instrument(name = "cache::get_or_insert_with", skip(self, key, f))]
    pub async fn get_or_insert_with<F, Fut, T>(
        &self,
        key: &str,
        f: F,
        expire: Option<u64>,
    ) -> Result<T, IntegrationOSError>
    where
        T: for<'de> Deserialize<'de> + Serialize + Send + Clone,
        F: FnOnce() -> Fut + Send,
        Fut: Future<Output = Result<T, IntegrationOSError>> + Send,
    {
        match self.get(key).await? {
            Some(entry) => {
                tracing::debug!("Cache hit for key: {}", key);
                Ok(entry)
            }
            None => {
                tracing::debug!("Cache miss for key: {}", key);
                let value = f().await?;
                self.set(key, value.clone(), expire).await?;
                Ok(value)
            }
        }
    }
}

impl RemoteCacheExt for RedisCache {
    #[tracing::instrument(name = "cache::get", skip(self, key))]
    async fn get<T>(&self, key: &str) -> Result<Option<T>, IntegrationOSError>
    where
        T: for<'de> Deserialize<'de>,
    {
        let entry: CacheEntry = self.connection().await?.get(key).await.map_err(|warn| {
            tracing::warn!("Error getting the value: {:?}", warn);
            InternalError::io_err("Error getting the value from the cache", None)
        })?;

        match entry.value {
            Some(_) => Ok(entry.get_as()),
            None => Ok(None),
        }
    }

    #[tracing::instrument(name = "cache::insert", skip(self, key, value, expire))]
    async fn set<T>(
        &self,
        key: &str,
        value: T,
        expire: Option<u64>,
    ) -> Result<Unit, IntegrationOSError>
    where
        T: Serialize + Send,
    {
        let entry = CacheEntry::new(
            key.to_string(),
            serde_json::to_value(value).map_err(|warn| {
                tracing::warn!("Error serializing the value: {:?}", warn);
                InternalError::io_err("Error serializing the value", None)
            })?,
        );

        self.connection()
            .await?
            .set_ex::<_, CacheEntry, Option<String>>(
                entry.key.clone(),
                entry.clone(),
                expire.unwrap_or(86400),
            )
            .await
            .map(|_| ())
            .map_err(|warn: RedisError| {
                tracing::warn!("Error setting the value: {:?}", warn);
                InternalError::io_err("Error setting the value in the cache", None)
            })?;

        Ok(())
    }

    #[tracing::instrument(name = "cache::remove", skip(self, key))]
    async fn remove(&self, key: &str) -> Result<Unit, IntegrationOSError> {
        self.connection()
            .await?
            .del::<_, i64>(key)
            .await
            .map(|_| ())
            .map_err(|warn: RedisError| {
                tracing::warn!("Error removing the value: {:?}", warn);
                InternalError::io_err("Error removing the value from the cache", None)
            })?;

        Ok(())
    }

    #[tracing::instrument(name = "cache::clear", skip(self))]
    async fn clear(&self) -> Result<Unit, IntegrationOSError> {
        cmd("FLUSHALL")
            .query_async(&mut *self.connection().await?)
            .await
            .map(|_: ()| ())
            .map_err(|warn: RedisError| {
                tracing::warn!("Error clearing the cache: {:?}", warn);
                InternalError::io_err("Error clearing the cache", None)
            })?;

        Ok(())
    }
}

// All credits to [absenty-cache](https://gist.github.com/samgj18/5b5a805050b694cc4092cd067c9c5048)
