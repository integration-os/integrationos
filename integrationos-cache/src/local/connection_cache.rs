use crate::LocalCacheExt;
use http::HeaderValue;
use integrationos_domain::{Connection, IntegrationOSError, MongoStore, Unit};
use moka::future::Cache;
use mongodb::bson::Document;
use std::fmt::Debug;
use std::hash::Hash;
use std::{sync::Arc, time::Duration};

#[derive(Clone)]
pub struct ConnectionCacheForKey<K: Clone + Send + Sync + Eq + Hash + Debug + 'static> {
    inner: Arc<Cache<K, Connection>>,
}

impl<K: Clone + Send + Sync + Eq + Hash + Debug + 'static> ConnectionCacheForKey<K> {
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
        store: MongoStore<Connection>,
        filter: Document,
    ) -> Result<Connection, IntegrationOSError> {
        self.inner
            .get_or_insert_with_filter(&key, store, filter)
            .await
    }

    pub async fn get(&self, key: K) -> Result<Option<Connection>, IntegrationOSError> {
        self.inner.get(&key).await
    }

    pub async fn set(&self, key: K, value: &Connection) -> Result<Unit, IntegrationOSError> {
        self.inner.set(&key, value).await
    }

    pub async fn remove(&self, key: K) -> Result<Unit, IntegrationOSError> {
        self.inner.remove(&key).await
    }
}

pub type ConnectionCacheArcStrKey = ConnectionCacheForKey<Arc<str>>;

impl ConnectionCacheArcStrKey {
    pub fn create(size: u64, ttl: u64) -> ConnectionCacheForKey<Arc<str>> {
        ConnectionCacheForKey::new(size, ttl)
    }
}

pub type ConnectionCacheArcStrHeaderKey = ConnectionCacheForKey<(Arc<str>, HeaderValue)>;

impl ConnectionCacheArcStrHeaderKey {
    pub fn create(size: u64, ttl: u64) -> ConnectionCacheForKey<(Arc<str>, HeaderValue)> {
        ConnectionCacheForKey::new(size, ttl)
    }
}
