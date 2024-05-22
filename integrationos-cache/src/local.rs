use crate::LocalCacheExt;
use http::HeaderValue;
use integrationos_domain::{
    connection_model_definition::ConnectionModelDefinition, event_access::EventAccess, Connection,
    Id, IntegrationOSError, MongoStore, Unit,
};
use moka::future::Cache;
use mongodb::bson::Document;
use serde::{de::DeserializeOwned, Serialize};
use std::{fmt::Debug, hash::Hash, sync::Arc, time::Duration};

impl<K, V> LocalCacheExt<K, V> for Arc<Cache<K, V>>
where
    K: Hash + Eq + Clone + Debug + Send + Sync + 'static,
    V: Clone + DeserializeOwned + Send + Sync + Unpin + Serialize + 'static,
{
    async fn get(&self, key: &K) -> Result<Option<V>, IntegrationOSError> {
        match Cache::get(self, key).await {
            Some(entry) => Ok(Some(entry)),
            None => Ok(None),
        }
    }

    async fn set(&self, key: &K, value: &V) -> Result<Unit, IntegrationOSError> {
        Cache::insert(self, key.clone(), value.clone()).await;
        Ok(())
    }

    async fn remove(&self, key: &K) -> Result<Unit, IntegrationOSError> {
        Cache::remove(self, key).await;
        Ok(())
    }
}

pub struct ConnectionCache {
    inner: Arc<Cache<Id, Connection>>,
}

impl ConnectionCache {
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
        store: MongoStore<Connection>,
        filter: Document,
    ) -> Result<Connection, IntegrationOSError> {
        self.inner
            .get_or_insert_with_filter(key, store, filter)
            .await
    }

    pub async fn get(&self, key: &Id) -> Result<Option<Connection>, IntegrationOSError> {
        self.inner.get(key).await
    }

    pub async fn set(&self, key: &Id, value: &Connection) -> Result<Unit, IntegrationOSError> {
        self.inner.set(key, value).await
    }

    pub async fn remove(&self, key: &Id) -> Result<Unit, IntegrationOSError> {
        self.inner.remove(key).await
    }
}

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

pub struct EventAccessCache {
    inner: Arc<Cache<HeaderValue, EventAccess>>,
}

impl EventAccessCache {
    pub fn new(size: u64) -> Self {
        Self {
            inner: Arc::new(Cache::builder().max_capacity(size).build()),
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

#[cfg(test)]
mod tests {
    use super::*;
    use fake::{Fake, Faker};
    use mongodb::bson::doc;
    use serde::Deserialize;
    use std::time::Duration;

    #[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
    pub struct Element {
        pub id: String,
        pub value: String,
    }

    pub struct TestCache {
        inner: Arc<Cache<String, Element>>,
    }

    impl TestCache {
        pub fn new(size: u64, ttl: Duration) -> Self {
            Self {
                inner: Arc::new(
                    Cache::builder()
                        .max_capacity(size)
                        .time_to_live(ttl)
                        .build(),
                ),
            }
        }

        pub async fn get(&self, key: &String) -> Result<Option<Element>, IntegrationOSError> {
            self.inner.get(key).await
        }

        pub async fn set(&self, key: &String, value: &Element) -> Result<Unit, IntegrationOSError> {
            self.inner.set(key, value).await
        }

        pub async fn remove(&self, key: &String) -> Result<Unit, IntegrationOSError> {
            self.inner.remove(key).await
        }
    }

    #[tokio::test]
    async fn test_local_cache() {
        let cache = TestCache::new(10, Duration::from_secs(3));
        println!("Policy: {:?}", cache.inner.policy());

        let id = Faker.fake();
        let value = Faker.fake();
        let element = Element { id, value };

        let different_id = Faker.fake();

        let result = cache.get(&different_id).await.expect("get failed");
        assert_eq!(result, None);

        cache
            .set(&different_id, &element)
            .await
            .expect("set failed");

        let result = cache.get(&different_id).await.expect("get failed");
        assert_eq!(result, Some(element.clone()));

        cache.remove(&different_id).await.expect("remove failed");

        let result = cache.get(&different_id).await.expect("get failed");
        assert_eq!(result, None);

        // Test expiry
        let id = Faker.fake();
        let value = Faker.fake();
        let element = Element { id, value };

        cache.set(&element.id, &element).await.expect("set failed");

        let result = cache.get(&element.id).await.expect("get failed");
        assert_eq!(result, Some(element.clone()));

        // wait for three seconds
        tokio::time::sleep(Duration::from_secs(5)).await;

        let result = cache.get(&element.id).await.expect("get failed");
        assert_eq!(result, None);
    }
}
