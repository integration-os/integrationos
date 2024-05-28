use crate::LocalCacheExt;
use http::HeaderValue;
use integrationos_domain::{event_access::EventAccess, IntegrationOSError, MongoStore, Unit};
use moka::future::Cache;
use mongodb::bson::Document;
use std::{sync::Arc, time::Duration};

#[derive(Clone)]
pub struct EventAccessCache {
    inner: Arc<Cache<HeaderValue, EventAccess>>,
}

impl EventAccessCache {
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
        key: &HeaderValue,
        store: MongoStore<EventAccess>,
        filter: Document,
    ) -> Result<EventAccess, IntegrationOSError> {
        self.inner
            .get_or_insert_with_filter(key, store, filter)
            .await
    }

    pub async fn get(&self, key: &HeaderValue) -> Result<Option<EventAccess>, IntegrationOSError> {
        self.inner.get(key).await
    }

    pub async fn set(
        &self,
        key: &HeaderValue,
        value: &EventAccess,
    ) -> Result<Unit, IntegrationOSError> {
        self.inner.set(key, value).await
    }

    pub async fn remove(&self, key: &HeaderValue) -> Result<Unit, IntegrationOSError> {
        self.inner.remove(key).await
    }
}
