use crate::LocalCacheExt;
use futures::Future;
use integrationos_domain::{Connection, IntegrationOSError, InternalError, MongoStore, Unit};
use moka::future::Cache;
use mongodb::bson::Document;
use serde_json::Value;
use std::{sync::Arc, time::Duration};

#[derive(Clone)]
pub struct SecretCache {
    inner: Arc<Cache<Connection, Value>>,
}

impl SecretCache {
    pub fn new(size: u64, ttl: u64) -> Self {
        Self {
            inner: Arc::new(
                Cache::builder()
                    .max_capacity(size)
                    .time_to_live(Duration::from_secs(ttl))
                    .build(),
            ),
        }
    }

    pub fn max_capacity(&self) -> u64 {
        self.inner.policy().max_capacity().unwrap_or_default()
    }

    pub async fn get_or_insert_with_filter(
        &self,
        _: &Connection,
        _: MongoStore<Value>,
        _: Document,
    ) -> Result<Value, IntegrationOSError> {
        Err(InternalError::key_not_found(
            "The method you are trying to use is not implemented for this cache",
            None,
        ))
    }

    pub async fn get_or_insert_with_fn<F, Fut>(
        &self,
        key: Connection,
        fa: F,
    ) -> Result<Value, IntegrationOSError>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<Value, IntegrationOSError>>,
    {
        let result = self.inner.get(&key).await;
        match result {
            Ok(Some(value)) => Ok(value),
            Ok(None) => {
                let value = fa().await?;
                self.inner.set(&key, &value).await?;
                Ok(value)
            }
            Err(e) => Err(e),
        }
    }

    pub async fn get(&self, key: &Connection) -> Result<Option<Value>, IntegrationOSError> {
        self.inner.get(key).await
    }

    pub async fn set(&self, key: &Connection, value: &Value) -> Result<Unit, IntegrationOSError> {
        self.inner.set(key, value).await
    }

    pub async fn remove(&self, key: &Connection) -> Result<Unit, IntegrationOSError> {
        self.inner.remove(key).await
    }
}
