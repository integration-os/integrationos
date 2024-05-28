// pub type InMemoryCache<T> = Arc<Cache<Option<BTreeMap<String, String>>, Arc<T>>>;
use crate::LocalCacheExt;
use integrationos_domain::{
    connection_definition::ConnectionDefinition, Id, IntegrationOSError, MongoStore, Unit,
};
use moka::future::Cache;
use mongodb::bson::Document;
use std::{sync::Arc, time::Duration};

pub struct ConnectionDefinitionCache {
    inner: Arc<Cache<Id, ConnectionDefinition>>,
}

impl ConnectionDefinitionCache {
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
        key: &Id,
        store: MongoStore<ConnectionDefinition>,
        filter: Document,
    ) -> Result<ConnectionDefinition, IntegrationOSError> {
        self.inner
            .get_or_insert_with_filter(key, store, filter)
            .await
    }

    pub async fn get(&self, key: &Id) -> Result<Option<ConnectionDefinition>, IntegrationOSError> {
        self.inner.get(key).await
    }

    pub async fn set(
        &self,
        key: &Id,
        value: &ConnectionDefinition,
    ) -> Result<Unit, IntegrationOSError> {
        self.inner.set(key, value).await
    }

    pub async fn remove(&self, key: &Id) -> Result<Unit, IntegrationOSError> {
        self.inner.remove(key).await
    }
}
