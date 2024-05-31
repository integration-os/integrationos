use crate::LocalCacheExt;
use futures::Future;
use integrationos_domain::{
    connection_model_definition::ConnectionModelDefinition, destination::Destination, Id,
    IntegrationOSError, MongoStore, Unit,
};
use moka::future::Cache;
use mongodb::bson::Document;
use std::{fmt::Debug, hash::Hash, sync::Arc, time::Duration};

#[derive(Clone)]
pub struct ConnectionModelDefinitionCacheForKey<
    K: Clone + Send + Sync + Eq + Hash + Debug + 'static,
> {
    inner: Arc<Cache<K, ConnectionModelDefinition>>,
}

impl<K: Clone + Send + Sync + Eq + Hash + Debug + 'static> ConnectionModelDefinitionCacheForKey<K> {
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
        key: K,
        store: MongoStore<ConnectionModelDefinition>,
        filter: Document,
    ) -> Result<ConnectionModelDefinition, IntegrationOSError> {
        self.inner
            .get_or_insert_with_filter(&key, store, filter)
            .await
    }

    pub async fn get_or_insert_with_fn<F, Fut>(
        &self,
        key: K,
        fa: F,
    ) -> Result<ConnectionModelDefinition, IntegrationOSError>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<ConnectionModelDefinition, IntegrationOSError>>,
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

    pub async fn get(
        &self,
        key: K,
    ) -> Result<Option<ConnectionModelDefinition>, IntegrationOSError> {
        self.inner.get(&key).await
    }

    pub async fn set(
        &self,
        key: K,
        value: &ConnectionModelDefinition,
    ) -> Result<Unit, IntegrationOSError> {
        self.inner.set(&key, value).await
    }

    pub async fn remove(&self, key: K) -> Result<Unit, IntegrationOSError> {
        self.inner.remove(&key).await
    }
}

#[derive(Clone)]
pub struct ConnectionModelDefinitionCacheIdKey;

impl ConnectionModelDefinitionCacheIdKey {
    pub fn create(size: u64, ttl: u64) -> ConnectionModelDefinitionCacheForKey<Id> {
        ConnectionModelDefinitionCacheForKey::new(size, ttl)
    }
}

pub type ConnectionModelDefinitionDestinationKey =
    ConnectionModelDefinitionCacheForKey<Destination>;

impl ConnectionModelDefinitionDestinationKey {
    pub fn create(size: u64, ttl: u64) -> ConnectionModelDefinitionCacheForKey<Destination> {
        ConnectionModelDefinitionCacheForKey::new(size, ttl)
    }
}
