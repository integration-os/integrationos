pub mod local;
pub mod remote;

use futures::Future;
use integrationos_domain::{ApplicationError, IntegrationOSError, MongoStore, Unit};
use mongodb::bson::Document;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::hash::Hash;

pub trait RemoteCacheExt {
    fn get<T>(
        &self,
        key: &str,
    ) -> impl Future<Output = Result<Option<T>, IntegrationOSError>> + Send
    where
        T: for<'de> Deserialize<'de>;
    fn set<T>(
        &self,
        key: &str,
        value: T,
        expire: Option<u64>,
    ) -> impl Future<Output = Result<Unit, IntegrationOSError>> + Send
    where
        T: Serialize + Send;
    fn remove(&self, key: &str) -> impl Future<Output = Result<Unit, IntegrationOSError>> + Send;
    fn clear(&self) -> impl Future<Output = Result<Unit, IntegrationOSError>> + Send;
}

pub trait LocalCacheExt<K, V>
where
    K: Hash + Eq + Clone + Debug,
    V: Clone + DeserializeOwned + Send + Sync + Unpin + Serialize + 'static,
{
    fn get_or_insert_with_filter(
        &self,
        key: &K,
        store: MongoStore<V>,
        filter: Document,
    ) -> impl Future<Output = Result<V, IntegrationOSError>> {
        async move {
            match self.get(key).await? {
                Some(entry) => {
                    tracing::debug!("Cache hit for key: {:?}", key);
                    Ok(entry)
                }
                None => {
                    tracing::debug!("Cache miss for key: {:?}", key);
                    let value = store.get_one(filter).await?;
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
    }

    fn get(&self, key: &K) -> impl Future<Output = Result<Option<V>, IntegrationOSError>>;

    fn set(&self, key: &K, value: &V) -> impl Future<Output = Result<Unit, IntegrationOSError>>;

    fn remove(&self, key: &K) -> impl Future<Output = Result<Unit, IntegrationOSError>>;
}
