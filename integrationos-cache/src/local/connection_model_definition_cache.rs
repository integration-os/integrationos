use crate::LocalCacheExt;
use integrationos_domain::{
    connection_model_definition::ConnectionModelDefinition, Id, IntegrationOSError, MongoStore,
    Unit,
};
use moka::future::Cache;
use mongodb::bson::Document;
use std::{sync::Arc, time::Duration};

#[derive(Clone)]
pub struct ConnectionModelDefinitionCache {
    inner: Arc<Cache<Id, ConnectionModelDefinition>>,
}

impl ConnectionModelDefinitionCache {
    pub fn new(size: u64) -> Self {
        Self {
            inner: Arc::new(
                Cache::builder()
                    .max_capacity(size)
                    .time_to_live(Duration::from_secs(120))
                    .build(),
            ),
        }
    }

    pub async fn get_or_insert_with_filter(
        &self,
        key: &Id,
        store: MongoStore<ConnectionModelDefinition>,
        filter: Document,
    ) -> Result<ConnectionModelDefinition, IntegrationOSError> {
        self.inner
            .get_or_insert_with_filter(key, store, filter)
            .await
    }

    pub async fn get(
        &self,
        key: &Id,
    ) -> Result<Option<ConnectionModelDefinition>, IntegrationOSError> {
        self.inner.get(key).await
    }

    pub async fn set(
        &self,
        key: &Id,
        value: &ConnectionModelDefinition,
    ) -> Result<Unit, IntegrationOSError> {
        self.inner.set(key, value).await
    }

    pub async fn remove(&self, key: &Id) -> Result<Unit, IntegrationOSError> {
        self.inner.remove(key).await
    }
}
