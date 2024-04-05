use crate::{config::Config, finalize_event::FinalizeEvent};
use anyhow::{bail, Context, Result};
use async_trait::async_trait;
use integrationos_domain::{
    algebra::adapter::StoreAdapter,
    common::{
        encrypted_access_key::EncryptedAccessKey,
        event_with_context::EventWithContext,
        mongo::{MongoDbStore, MongoDbStoreConfig},
        Event, RootContext, Store,
    },
};
use mongodb::Collection;
use redis_retry::{AsyncCommands, Redis};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, error};

pub struct Finalizer {
    redis: Arc<Mutex<Redis>>,
    context_collection: Collection<RootContext>,
    event_store: MongoDbStore<Event>,
    queue_name: String,
}

impl Finalizer {
    pub async fn new(config: Config) -> Result<Self> {
        let redis = Redis::new_with_retry_count(&config.redis, 2).await?;

        let context_mongo_client = mongodb::Client::with_uri_str(config.db.context_db_url)
            .await
            .with_context(|| "Could not connect to context mongodb")?;
        let context_db = context_mongo_client.database(&config.db.context_db_name);
        let context_collection = context_db.collection(&config.db.context_collection_name);

        let mongo = mongodb::Client::with_uri_str(config.db.event_db_url)
            .await
            .with_context(|| "Could not connect to mongodb")?;
        let mongo = mongo.database(&config.db.event_db_name);
        let event_store = MongoDbStore::new(MongoDbStoreConfig::new(mongo, Store::Events))
            .await
            .with_context(|| {
                format!(
                    "Could not connect to event db at {}",
                    config.db.event_db_name
                )
            })?;
        Ok(Self {
            redis: Arc::new(Mutex::new(redis)),
            context_collection,
            event_store,
            queue_name: config.redis.queue_name,
        })
    }
}

#[async_trait]
impl FinalizeEvent for Finalizer {
    async fn finalize_event(
        &self,
        event: &Event,
        _event_name: &str,
        _access_key: &EncryptedAccessKey,
    ) -> Result<String, anyhow::Error> {
        match self.event_store.create_one(event).await {
            Err(e) => {
                error!("Failed to save event: {e}");
                bail!(e);
            }
            Ok(r) => {
                debug!("Inserted event {event:?} => result for insertion {r:?}");
            }
        }
        let context = RootContext::new(event.id);
        match self.context_collection.insert_one(&context, None).await {
            Err(e) => {
                error!("Failed to save event context: {e}");
                bail!(e);
            }
            Ok(r) => {
                debug!("Inserted event context {context:?} => result for insertion {r:?}");
            }
        }

        let msg = EventWithContext::new(event.clone(), context);
        let msg: Vec<u8> = serde_json::to_vec(&msg)?;
        let mut conn = self.redis.lock().await;
        match conn.lpush(&self.queue_name, &msg).await {
            Ok(()) => Ok("Sent on redis".to_string()),
            Err(e) => {
                error!("Could not publish to redis: {e}");
                bail!(e);
            }
        }
    }
}
