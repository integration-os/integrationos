use crate::config::WatchdogConfig;
use bson::{doc, Bson, Document};
use chrono::Utc;
use futures::{future::join_all, TryStreamExt};
use integrationos_cache::remote::RedisCache;
use integrationos_domain::{
    cache::CacheConfig, database::DatabaseConfig, event_with_context::EventWithContext,
    pipeline_context::PipelineStage, prelude::MongoStore, root_context::RootStage, Event,
    ExtractorContext, IntegrationOSError, InternalError, PipelineContext, RootContext, Store,
};
use mongodb::options::FindOneOptions;
use redis::{AsyncCommands, LposOptions, RedisResult};
use std::fmt::Display;
use std::time::Duration;
use tokio::task::JoinHandle;
use tracing::{error, info, warn};

pub struct WatchdogClient {
    watchdog: WatchdogConfig,
    cache: CacheConfig,
    database: DatabaseConfig,
}

impl Display for WatchdogClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let cache = format!("{}", self.cache);
        let database = format!("{}", self.database);
        let watchdog = format!("{}", self.watchdog);

        write!(
            f,
            "WatchdogClient {{ watchdog: {watchdog}, cache: {cache}, database: {database} }}",
        )
    }
}

impl WatchdogClient {
    pub fn new(watchdog: WatchdogConfig, cache: CacheConfig, database: DatabaseConfig) -> Self {
        Self {
            watchdog,
            cache,
            database,
        }
    }

    pub fn start(self) -> JoinHandle<Result<(), IntegrationOSError>> {
        tokio::spawn(self.run())
    }

    pub async fn run(self) -> Result<(), IntegrationOSError> {
        info!("Starting watchdog");
        let mut cache = RedisCache::new(&self.cache).await.map_err(|e| {
            error!("Could not connect to cache: {e}");
            InternalError::io_err(e.to_string().as_str(), None)
        })?;
        let key = self.cache.event_throughput_key.clone();

        info!("Initializing connection to cache");

        let mut redis_clone = cache.inner.clone();
        tokio::spawn(async move {
            loop {
                let _: RedisResult<String> = async { redis_clone.del(key.clone()).await }.await;
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        });

        let key = self.cache.api_throughput_key.clone();
        let mut redis_clone = cache.inner.clone();
        tokio::spawn(async move {
            loop {
                let _: RedisResult<String> = async { redis_clone.del(key.clone()).await }.await;
                tokio::time::sleep(Duration::from_secs(60)).await;
            }
        });

        info!("Initialized connection to cache");
        info!("Intializing connection to storage");

        let mongo = mongodb::Client::with_uri_str(self.database.context_db_url.clone())
            .await
            .map_err(|e| {
                error!("Could not connect to mongodb: {e}");
                InternalError::io_err(e.to_string().as_str(), None)
            })?;
        let db = mongo.database(&self.database.context_db_name);
        let coll = db.collection::<Document>(&self.database.context_collection_name);
        let root_coll = db.collection::<RootContext>(&self.database.context_collection_name);
        let pipeline_coll =
            db.collection::<PipelineContext>(&self.database.context_collection_name);
        let extractor_coll =
            db.collection::<ExtractorContext>(&self.database.context_collection_name);
        let event_client = mongodb::Client::with_uri_str(self.database.event_db_url.clone())
            .await
            .map_err(|e| {
                error!("Could not connect to events db: {e}");
                InternalError::io_err(e.to_string().as_str(), None)
            })?;

        let event_db = event_client.database(&self.database.event_db_name);
        let event_store: MongoStore<Event> = MongoStore::new(&event_db, &Store::Events)
            .await
            .map_err(|e| {
                error!("Could not connect to event db: {e}");
                InternalError::io_err(e.to_string().as_str(), None)
            })?;

        info!("Initialized connection to storage");

        loop {
            info!("Polling for unresponsive contexts");
            let mut count = 0;
            let timestamp =
                Utc::now().timestamp_millis() - (self.watchdog.event_timeout * 1_000) as i64;

            let pipeline = vec![
                // Sort by timestamp to get latest contexts first
                doc! {
                  "$sort": {
                    "timestamp": -1
                  },
                },
                // Group by event_key
                // Get the first (latest) context's stage and status
                // Count any contexts that are later than the poll duration cutoff
                // If there are any that are later then this context is still not dead
                doc! {
                  "$group": {
                    "_id": "$eventKey",
                    "stage": {
                      "$first": "$stage"
                    },
                    "status": {
                        "$first": "$status"
                    },
                    "count": {
                      "$sum": {
                        "$cond": [{
                            "$gt": [
                              "$timestamp", timestamp
                            ]
                        }, 1, 0]
                      },
                    },
                  },
                },
                // Match any contexts that have no contexts after our cutoff date, so presumed dead
                // And also not finished and status is succeeded (not dropped)
                // These contexts are unfinished and dead, so need to be republished to redis
                doc! {
                  "$match": {
                    "count": { "$eq": 0 },
                    "stage": { "$ne": "Finished" },
                    "status": { "$eq": "Succeeded" }
                  }
                },
            ];

            let mut event_keys = match coll.clone().aggregate(pipeline, None).await {
                Ok(e) => e,
                Err(e) => {
                    error!("Failed to fetch event keys: {e}");
                    continue;
                }
            };

            info!("Fetched event keys");

            'outer: while let Some(event_key) = event_keys.try_next().await? {
                let Some(Bson::String(event_key)) = event_key.get("_id") else {
                    error!("Could not get _id out of event keys response");
                    continue;
                };
                // Sort by earliest timestamp to get latest context
                let options = FindOneOptions::builder()
                    .sort(doc! { "timestamp": -1 })
                    .build();

                // Get the latest root context, then also get all latest pipeline contexts and extractor contexts if applicable
                let root_context = match root_coll
                    .clone()
                    .find_one(
                        doc! {
                            "eventKey": event_key,
                            "type": "root"
                        },
                        options.clone(),
                    )
                    .await
                {
                    Ok(c) => c,
                    Err(e) => {
                        error!("Failed to fetch root context: {e}");
                        continue;
                    }
                };
                let Some(mut root_context) = root_context else {
                    error!("Did not find root context for {event_key}");
                    continue;
                };

                if let RootStage::ProcessingPipelines(ref mut pipelines) = root_context.stage {
                    let futs = pipelines.values().map(|p| {
                        pipeline_coll.find_one(
                            doc! {
                                "eventKey": p.event_key.to_string(),
                                "pipelineKey": p.pipeline_key.clone(),
                                "type": "pipeline"
                            },
                            options.clone(),
                        )
                    });

                    let results = join_all(futs).await;
                    for result in results {
                        match result {
                            Ok(context) => {
                                let Some(mut context) = context else {
                                    error!("Did not find pipeline context for {event_key}");
                                    continue 'outer;
                                };
                                if let PipelineStage::ExecutingExtractors(ref mut extractors) =
                                    context.stage
                                {
                                    let futs = extractors.values().map(|e| {
                                        let filter = doc! {
                                            "eventKey": e.event_key.to_string(),
                                            "pipelineKey": e.pipeline_key.clone(),
                                            "extractorKey": e.extractor_key.to_string(),
                                            "type": "extractor"
                                        };
                                        extractor_coll.find_one(filter, options.clone())
                                    });
                                    let results = join_all(futs).await;
                                    for result in results {
                                        match result {
                                            Ok(context) => {
                                                let Some(context) = context else {
                                                    error!("Did not find extractor context for {event_key}");
                                                    continue 'outer;
                                                };
                                                extractors
                                                    .insert(context.extractor_key.clone(), context);
                                            }
                                            Err(e) => {
                                                error!("Did not find extractor context for {event_key}: {e}");
                                                continue 'outer;
                                            }
                                        }
                                    }
                                }
                                pipelines.insert(context.pipeline_key.clone(), context);
                            }
                            Err(e) => {
                                error!("Could not fetch pipeline context for {event_key}: {e}");
                                continue 'outer;
                            }
                        }
                    }
                }

                info!("Republishing unresponsive context {event_key}");

                let Some(event) = event_store.get_one_by_id(event_key).await.map_err(|e| {
                    error!("Could not fetch event for context {event_key}: {e}");
                    InternalError::io_err(e.to_string().as_str(), None)
                })?
                else {
                    error!("Event does not exist {event_key}");
                    continue;
                };

                let event_with_context = EventWithContext::new(event, root_context);

                let payload = match serde_json::to_vec(&event_with_context) {
                    Ok(c) => c,
                    Err(e) => {
                        error!("Could not serialize payload {event_with_context:?}: {e}");
                        continue;
                    }
                };
                let matching_idx = cache
                    .inner
                    .lpos::<&str, &[u8], Option<isize>>(
                        &self.cache.queue_name,
                        &payload,
                        LposOptions::default(),
                    )
                    .await
                    .map_err(|e| {
                        error!("Could not check if context is already in redis: {e}");
                        InternalError::io_err(e.to_string().as_str(), None)
                    })?;

                if (matching_idx).is_some() {
                    warn!("Unresponsive context is already in redis {event_key}");
                    continue;
                }

                match cache.inner.lpush(&self.cache.queue_name, payload).await {
                    Ok(()) => count += 1,
                    Err(e) => error!("Could not publish event to redis: {e}"),
                }
            }

            if count > 0 {
                info!("Republished {count} new events");
            }

            info!("Sleeping for {} seconds", self.watchdog.poll_duration);
            tokio::time::sleep(Duration::from_secs(self.watchdog.poll_duration)).await;
        }
    }
}
