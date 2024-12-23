use futures::Future;
use http::HeaderValue;
use integrationos_domain::connection_definition::ConnectionDefinition;
use integrationos_domain::connection_model_definition::ConnectionModelDefinition;
use integrationos_domain::connection_model_schema::ConnectionModelSchema;
use integrationos_domain::connection_oauth_definition::ConnectionOAuthDefinition;
use integrationos_domain::destination::Destination;
use integrationos_domain::event_access::EventAccess;
use integrationos_domain::{
    ApplicationError, Connection, Id, IntegrationOSError, MongoStore, Secret, Unit,
};
use moka::future::Cache;
use mongodb::bson::Document;
use mongodb::options::FindOneOptions;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::fmt::Debug;
use std::hash::Hash;
use std::sync::Arc;
use std::time::Duration;

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
        options: Option<FindOneOptions>,
    ) -> impl Future<Output = Result<V, IntegrationOSError>> {
        async move {
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
                        self.insert(key, &value).await?;
                        Ok(value)
                    } else {
                        tracing::warn!("Value with id {:?} not found", key);
                        Err(ApplicationError::not_found("Value not found", None))
                    }
                }
            }
        }
    }

    fn get_or_insert_with_fn<F, Fut>(
        &self,
        key: &K,
        fa: F,
    ) -> impl Future<Output = Result<V, IntegrationOSError>>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<V, IntegrationOSError>>,
    {
        async move {
            match self.get(key).await? {
                Some(entry) => {
                    tracing::debug!("Cache hit for key: {:?}", key);
                    Ok(entry)
                }
                None => {
                    let value = fa().await?;
                    self.insert(key, &value).await?;
                    Ok(value)
                }
            }
        }
    }

    fn get(&self, key: &K) -> impl Future<Output = Result<Option<V>, IntegrationOSError>>;

    fn insert(&self, key: &K, value: &V) -> impl Future<Output = Result<Unit, IntegrationOSError>>;

    fn remove(&self, key: &K) -> impl Future<Output = Result<Unit, IntegrationOSError>>;

    fn max_capacity(&self) -> u64;
}

#[derive(Clone)]
pub struct GenericCache<K, V>
where
    K: Hash + Eq + Clone + Debug,
    V: Clone + DeserializeOwned + Send + Sync + Unpin + Serialize + 'static,
{
    inner: Arc<Cache<K, V>>,
}

impl<K, V> GenericCache<K, V>
where
    K: Hash + Eq + Clone + Debug + Sync + Send + 'static,
    V: Clone + DeserializeOwned + Send + Sync + Unpin + Serialize + 'static,
{
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
}

impl<K, V> LocalCacheExt<K, V> for GenericCache<K, V>
where
    K: Hash + Eq + Clone + Debug + Sync + Send + 'static,
    V: Clone + DeserializeOwned + Send + Sync + Unpin + Serialize + 'static,
{
    async fn get(&self, key: &K) -> Result<Option<V>, IntegrationOSError> {
        let inner = self.inner.clone();
        Ok(inner.get(key).await)
    }

    async fn insert(&self, key: &K, value: &V) -> Result<Unit, IntegrationOSError> {
        let inner = self.inner.clone();
        inner.insert(key.clone(), value.clone()).await;
        Ok(())
    }

    async fn remove(&self, key: &K) -> Result<Unit, IntegrationOSError> {
        let inner = self.inner.clone();
        inner.remove(key).await;
        Ok(())
    }

    fn max_capacity(&self) -> u64 {
        self.inner.policy().max_capacity().unwrap_or_default()
    }
}

type ConnectionModelSchemaKey = (Arc<str>, Arc<str>);
type ConnectionHeaderKey = (Arc<str>, HeaderValue);
type ConnectionKey = Arc<str>;

pub type EventAccessCache = GenericCache<HeaderValue, EventAccess>;
pub type SecretCache = GenericCache<Connection, Secret>;
pub type ConnectionOAuthDefinitionCache = GenericCache<Id, ConnectionOAuthDefinition>;
pub type ConnectionModelSchemaCache = GenericCache<ConnectionModelSchemaKey, ConnectionModelSchema>;
pub type ConnectionModelDefinitionDestinationCache =
    GenericCache<Destination, ConnectionModelDefinition>;
pub type ConnectionModelDefinitionCacheIdKey = GenericCache<Id, ConnectionModelDefinition>;
pub type ConnectionDefinitionCache = GenericCache<Id, ConnectionDefinition>;
pub type ConnectionHeaderCache = GenericCache<ConnectionHeaderKey, Connection>;
pub type ConnectionCache = GenericCache<ConnectionKey, Connection>;
