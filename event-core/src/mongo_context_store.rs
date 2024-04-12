use crate::{config::EventCoreConfig, store::ContextStore};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use bson::doc;
use integrationos_domain::{algebra::PipelineExt, id::Id};
use mongodb::{Client, Database};
use serde::{Deserialize, Serialize};
use std::time::Instant;
use tracing::{error, trace};

#[derive(Debug, Clone)]
pub struct MongoContextStore {
    pub db: Database,
    pub collection_name: String,
}

impl MongoContextStore {
    pub async fn new(config: &EventCoreConfig) -> Result<Self> {
        let client = Client::with_uri_str(&config.db.context_db_url).await?;
        Ok(Self {
            db: client.database(&config.db.context_db_name),
            collection_name: config.db.context_collection_name.clone(),
        })
    }
}

#[async_trait]
impl ContextStore for MongoContextStore {
    async fn get<T: PipelineExt + Clone + for<'a> Deserialize<'a> + Unpin>(
        &self,
        context_key: &Id,
    ) -> Result<T> {
        let coll = self.db.collection(&self.collection_name);
        let context = coll
            .find_one(doc! { "id": context_key.to_string() }, None)
            .await?;
        Ok(context.ok_or_else(|| anyhow!("No context found"))?)
    }

    async fn set<T: PipelineExt + Clone + Serialize>(&self, context: T) -> Result<()> {
        let instant = Instant::now();
        let coll = self.db.collection(&self.collection_name);
        if let Err(e) = coll.insert_one(context, None).await {
            error!("PipelineExt insertion error {e}");
        }
        trace!(
            "Wrote context in {}",
            (Instant::now() - instant).as_millis()
        );
        Ok(())
    }
}
