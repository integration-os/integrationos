pub mod connection_cache;
pub mod connection_definition_cache;
pub mod connection_model_definition_cache;
pub mod connection_model_schema_cache;
pub mod connection_oauth_definition_cache;
pub mod event_access_cache;
pub mod secrets_cache;

use crate::LocalCacheExt;
use integrationos_domain::{IntegrationOSError, Unit};
use moka::future::Cache;
use serde::{de::DeserializeOwned, Serialize};
use std::{fmt::Debug, hash::Hash, sync::Arc};

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
