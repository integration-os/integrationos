use anyhow::{Context, Result};
use chrono::Utc;
use dotenvy::dotenv;
use envconfig::Envconfig;
use futures::{future::join_all, TryStreamExt};
use integrationos_domain::{
    algebra::adapter::StoreAdapter,
    common::{
        event_with_context::EventWithContext,
        mongo::{MongoDbStore, MongoDbStoreConfig},
        pipeline_context::Stage as PipelineStage,
        root_context::Stage,
        Event, ExtractorContext, PipelineContext, RootContext, Store,
    },
};
use mongodb::{
    bson::{doc, Bson, Document},
    options::FindOneOptions,
};
use redis_retry::{AsyncCommands, LposOptions, Redis, RedisResult};
use std::time::Duration;
use tracing::{debug, error, info, metadata::LevelFilter, warn};
use tracing_subscriber::EnvFilter;
use watchdog::config::Config;

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<()> {
    dotenv().ok();

    let filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::DEBUG.into())
        .from_env_lossy();
    tracing_subscriber::fmt().with_env_filter(filter).init();

    let config = Config::init_from_env()?;

    info!("Starting watchdog with config: {config}");

    let mut redis = Redis::new(&config.redis).await?;

    let key = config.redis.event_throughput_key.clone();
    let mut redis_clone = redis.clone();
    tokio::spawn(async move {
        loop {
            let _: RedisResult<String> = async { redis_clone.del(key.clone()).await }.await;
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    });

    let key = config.redis.api_throughput_key.clone();
    let mut redis_clone = redis.clone();
    tokio::spawn(async move {
        loop {
            let _: RedisResult<String> = async { redis_clone.del(key.clone()).await }.await;
            tokio::time::sleep(Duration::from_secs(60)).await;
        }
    });

    let mongo = mongodb::Client::with_uri_str(config.db.context_db_url)
        .await
        .with_context(|| "Could not connect to mongodb")?;
    let db = mongo.database(&config.db.context_db_name);
    let coll = db.collection::<Document>(&config.db.context_collection_name);
    let root_coll = db.collection::<RootContext>(&config.db.context_collection_name);
    let pipeline_coll = db.collection::<PipelineContext>(&config.db.context_collection_name);
    let extractor_coll = db.collection::<ExtractorContext>(&config.db.context_collection_name);

    let event_client = mongodb::Client::with_uri_str(config.db.event_db_url)
        .await
        .with_context(|| "Could not connect to events db")?;

    let event_db = event_client.database(&config.db.event_db_name);
    let event_store = MongoDbStore::new(MongoDbStoreConfig::<Event>::new(event_db, Store::Events))
        .await
        .with_context(|| {
            format!(
                "Could not connect to event db at {}",
                config.db.event_db_name
            )
        })?;

    loop {
        let mut count = 0;
        let timestamp = Utc::now().timestamp_millis() - (config.event_timeout * 1_000) as i64;

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

            if let Stage::ProcessingPipelines(ref mut pipelines) = root_context.stage {
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

            debug!("Republishing unresponsive context {event_key}");

            let Some(event) = event_store
                .get_one_by_id(event_key)
                .await
                .with_context(|| "could not fetch event for context {event_key}")?
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
            if redis
                .lpos::<&str, &[u8], Option<isize>>(
                    &config.redis.queue_name,
                    &payload,
                    LposOptions::default(),
                )
                .await?
                .is_some()
            {
                warn!("Unresponsive context is already in redis {event_key}");
                continue;
            }
            match redis.lpush(&config.redis.queue_name, payload).await {
                Ok(()) => count += 1,
                Err(e) => error!("Could not publish event to redis: {e}"),
            }
        }

        if count > 0 {
            info!("Republished {count} new events");
        }

        tokio::time::sleep(Duration::from_secs(config.poll_duration)).await;
    }
}
