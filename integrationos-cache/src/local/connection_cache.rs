use crate::LocalCacheExt;
use http::HeaderValue;
use integrationos_domain::{Connection, IntegrationOSError, MongoStore, Unit};
use moka::future::Cache;
use mongodb::bson::Document;
use std::{sync::Arc, time::Duration};

#[derive(Clone)]
pub struct ConnectionCache {
    inner: Arc<Cache<(Arc<str>, HeaderValue), Connection>>,
}

impl ConnectionCache {
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
        key: &(Arc<str>, HeaderValue),
        store: MongoStore<Connection>,
        filter: Document,
    ) -> Result<Connection, IntegrationOSError> {
        self.inner
            .get_or_insert_with_filter(key, store, filter)
            .await
    }

    pub async fn get(
        &self,
        key: &(Arc<str>, HeaderValue),
    ) -> Result<Option<Connection>, IntegrationOSError> {
        self.inner.get(key).await
    }

    pub async fn set(
        &self,
        key: &(Arc<str>, HeaderValue),
        value: &Connection,
    ) -> Result<Unit, IntegrationOSError> {
        self.inner.set(key, value).await
    }

    pub async fn remove(&self, key: &(Arc<str>, HeaderValue)) -> Result<Unit, IntegrationOSError> {
        self.inner.remove(key).await
    }
}
