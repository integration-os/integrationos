use crate::LocalCacheExt;
use integrationos_domain::{
    connection_model_schema::ConnectionModelSchema, ApplicationError, IntegrationOSError,
    MongoStore, Unit,
};
use moka::future::Cache;
use mongodb::{bson::Document, options::FindOneOptions};
use std::{sync::Arc, time::Duration};

type ConnectionModelSchemaKey = (Arc<str>, Arc<str>);

#[derive(Clone)]
pub struct ConnectionModelSchemaCache {
    inner: Arc<Cache<ConnectionModelSchemaKey, ConnectionModelSchema>>,
}

impl ConnectionModelSchemaCache {
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

    pub async fn get_or_insert_with_filter(
        &self,
        key: &ConnectionModelSchemaKey,
        store: MongoStore<ConnectionModelSchema>,
        filter: Document,
        options: Option<FindOneOptions>,
    ) -> Result<ConnectionModelSchema, IntegrationOSError> {
        match self.get(key).await? {
            Some(entry) => {
                tracing::debug!("Cache hit for key: {:?}", key);
                Ok(entry)
            }
            None => {
                tracing::debug!("Cache miss for key: {:?}", key);
                let value = store
                    .collection
                    .find_one(filter)
                    .with_options(options)
                    .await?;
                if let Some(value) = value {
                    self.set(key, &value).await?;
                    Ok(value)
                } else {
                    tracing::warn!("Value with id {:?} not found", key);
                    Err(ApplicationError::not_found("Value not found", None))
                }
            }
        }
    }

    pub async fn get(
        &self,
        key: &ConnectionModelSchemaKey,
    ) -> Result<Option<ConnectionModelSchema>, IntegrationOSError> {
        self.inner.get(key).await
    }

    pub async fn set(
        &self,
        key: &ConnectionModelSchemaKey,
        value: &ConnectionModelSchema,
    ) -> Result<Unit, IntegrationOSError> {
        self.inner.set(key, value).await
    }

    pub async fn remove(&self, key: &ConnectionModelSchemaKey) -> Result<Unit, IntegrationOSError> {
        self.inner.remove(key).await
    }
}
