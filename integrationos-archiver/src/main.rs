mod config;
mod event;
mod storage;

use anyhow::{anyhow, Result};
use bson::{doc, Document};
use chrono::offset::LocalResult;
use chrono::{DateTime, Duration as CDuration, TimeZone, Utc};
use config::{ArchiverConfig, Mode};
use envconfig::Envconfig;
use event::completed::Completed;
use event::dumped::Dumped;
use event::failed::Failed;
use event::started::Started;
use event::uploaded::Uploaded;
use event::{Event, EventMetadata};
use futures::future::ready;
use futures::stream::{self, Stream};
use futures::StreamExt;
use integrationos_domain::telemetry::{get_subscriber, init_subscriber};
use integrationos_domain::{MongoStore, Store, Unit};
use mongodb::Client;
use std::process::Command;
use std::sync::Arc;
use storage::google_cloud::GoogleCloudStorage;
use storage::{Extension, Storage, StorageProvider};
use tempfile::TempDir;

#[tokio::main]
async fn main() -> Result<Unit> {
    let config = Arc::new(ArchiverConfig::init_from_env()?);
    let storage = Arc::new(match config.storage_provider {
        StorageProvider::GoogleCloud => GoogleCloudStorage::new(&config).await?,
    });

    let subscriber = get_subscriber("archiver".into(), "info".into(), std::io::stdout);
    init_subscriber(subscriber);

    tracing::info!("Starting archiver with config:\n{config}");

    let client = Arc::new(Client::with_uri_str(&config.db_config.event_db_url).await?);
    let database = Arc::new(client.database(&config.db_config.event_db_name));
    let archives: Arc<MongoStore<Event>> =
        Arc::new(MongoStore::new(&database, &Store::Archives).await?);

    let started = Started::new(config.event_collection_name.clone())?;
    let target_store: Arc<MongoStore<Document>> =
        Arc::new(MongoStore::new(&database, started.collection()).await?);
    archives
        .create_one(&Event::Started(started.clone()))
        .await?;

    // loop {
    match config.mode {
        Mode::Dump => dump(&config, &archives, &started, &storage, &target_store, false).await,
        Mode::DumpDelete => dump(&config, &archives, &started, &storage, &target_store, true).await,
        Mode::NoOp => Ok(()),
    }

    // tokio::time::sleep(Duration::from_secs(60)).await
    // }
}

async fn dump(
    config: &Arc<ArchiverConfig>,
    archives: &Arc<MongoStore<Event>>,
    started: &Started,
    storage: &Arc<impl Storage>,
    target_store: &Arc<MongoStore<Document>>,
    destructive: bool,
) -> Result<Unit> {
    tracing::info!(
        "Starting archiver in dump mode for the {} collection",
        started.collection()
    );

    let document = target_store
        .collection
        .find_one(
            doc! {},
            Some(
                mongodb::options::FindOneOptions::builder()
                    .sort(doc! { "arrivedAt": 1 }) // Sort by `arrivedAt` in ascending order
                    .projection(doc! { "arrivedAt": 1 }) // Only retrieve the `arrivedAt` field
                    .build(),
            ),
        )
        .await
        .map_err(|e| anyhow!("Failed to find first event in collection: {e}"))?;

    let start = match document {
        Some(document) => document
            .get_i64("arrivedAt")
            .map_err(|e| anyhow!("Failed to get arrivedAt from document: {e}"))?,
        None => return Err(anyhow!("No events found in collection")),
    };

    let start = match Utc.timestamp_millis_opt(start) {
        LocalResult::Single(date) => date,
        _ => return Err(anyhow!("Invalid timestamp")),
    };
    let end = Utc::now() - CDuration::days(config.min_date_days); // 30 days ago by default

    let chunks = start.divide_by_stream(CDuration::minutes(config.chunk_size_minutes), end); // Chunk size is 20 minutes by default

    let stream = chunks.map(|(start_time, end_time)| async move {
        tracing::info!(
            "Processing events between {} - ({}) and {} - ({})",
            start_time,
            start_time.timestamp_millis(),
            end_time,
            end_time.timestamp_millis()
        );
        let saved = save(
            config,
            archives,
            storage,
            target_store,
            started,
            &start_time,
            &end_time,
        )
        .await;

        match saved {
            Ok(0) => {
                tracing::warn!("No events found between {} and {}", start_time, end_time);
                return Ok(());
            }
            Ok(count) => {
                tracing::info!("Archive saved successfully, saved {} events", count);
            }
            Err(e) => {
                archives
                    .create_one(&Event::Failed(Failed::new(
                        e.to_string(),
                        started.reference(),
                        start_time,
                        end_time,
                    )))
                    .await?;

                tracing::error!("Failed to save archive: {e}");

                return Err(e);
            }
        };

        if destructive {
            tracing::warn!("Deleting old events as destructive mode is enabled");
            let filter = doc! {
                "arrivedAt": {
                    "$gte": start_time.timestamp_millis(),
                    "$lt": end_time.timestamp_millis()
                }
            };

            target_store.collection.delete_many(filter, None).await?;
            tracing::warn!("Old events deleted successfully");
        }
        Ok::<_, anyhow::Error>(())
    });

    let errors = stream
        .buffer_unordered(config.concurrent_chunks)
        .fold(Vec::new(), |mut acc, result| async move {
            if let Err(e) = result {
                acc.push(e);
            }
            acc
        })
        .await;

    if !errors.is_empty() {
        tracing::error!("Encountered {} errors during processing:", errors.len());
        for error in &errors {
            tracing::error!("Error: {:?}", error);
        }
    } else {
        tracing::info!("All chunks processed successfully.");
    }

    Ok(())
}

async fn save(
    config: &ArchiverConfig,
    archive: &MongoStore<Event>,
    storage: &Arc<impl Storage>,
    target_store: &MongoStore<Document>,
    started_event: &Started,
    start_time: &DateTime<Utc>,
    end_time: &DateTime<Utc>,
) -> Result<u64> {
    let tmp_dir = TempDir::new()?;
    let filter = doc! {
        "arrivedAt": {
            "$gte": start_time.timestamp_millis(),
            "$lt": end_time.timestamp_millis()
        }
    };
    let count = target_store
        .collection
        .count_documents(filter.clone(), None)
        .await?;

    tracing::info!(
        "Found {} events between {} - ({}) and {} - ({})",
        count,
        start_time,
        start_time.timestamp_millis(),
        end_time,
        end_time.timestamp_millis()
    );

    if count == 0 {
        return Ok(0);
    }

    let command = Command::new("mongodump")
        .arg("--uri")
        .arg(&config.db_config.event_db_url)
        .arg("--db")
        .arg(&config.db_config.event_db_name)
        .arg("--collection")
        .arg(&config.event_collection_name)
        .arg("--query")
        .arg(serde_json::to_string(&filter)?)
        .arg("--out")
        .arg(tmp_dir.path())
        .arg("--gzip")
        .output()?;

    if !command.status.success() {
        return Err(anyhow!("Command mongodump failed: {:?}", command));
    }

    archive
        .create_one(&Event::Dumped(Dumped::new(started_event.reference())))
        .await?;

    let base_path = tmp_dir
        .path()
        .join(&config.db_config.event_db_name)
        .join(&config.event_collection_name);

    let suffix = format!(
        "{}-{}",
        start_time.timestamp_millis(),
        end_time.timestamp_millis()
    );

    if let Err(e) = storage
        .upload_file(&base_path, &Extension::Bson, config, suffix.clone())
        .await
    {
        return Err(anyhow!("Failed to upload bson file: {e}"));
    }

    archive
        .create_one(&Event::Uploaded(Uploaded::new(started_event.reference())))
        .await?;

    if let Err(e) = storage
        .upload_file(&base_path, &Extension::Metadata, config, suffix)
        .await
    {
        return Err(anyhow!("Failed to upload json file: {e}"));
    }

    let remote_path = format!("gs://{}{}", config.gs_storage_bucket, base_path.display());

    archive
        .create_one(&Event::Completed(Completed::new(
            remote_path.clone(),
            started_event.reference(),
        )))
        .await?;

    tracing::info!(
        "Archive completed at {}, saved to {} with reference {} for events between {} and {}",
        Utc::now(),
        remote_path,
        started_event.reference(),
        start_time,
        end_time
    );

    Ok(count)
}

pub trait DivideBy {
    fn divide_by_stream(
        &self,
        duration: CDuration,
        end: DateTime<Utc>,
    ) -> Box<dyn Stream<Item = (DateTime<Utc>, DateTime<Utc>)> + Unpin>;
}

impl DivideBy for DateTime<Utc> {
    fn divide_by_stream(
        &self,
        duration: CDuration,
        end: DateTime<Utc>,
    ) -> Box<dyn Stream<Item = (DateTime<Utc>, DateTime<Utc>)> + Unpin> {
        let current_start = *self;
        let stream = stream::unfold(current_start, move |start| {
            let next_end = start + duration;

            // If the next end is past the provided `end`, use the provided `end` as the cap
            let actual_end = if next_end > end { end } else { next_end };

            if start >= end {
                // Stop the stream when the start exceeds the given `end`
                ready(None)
            } else {
                ready(Some(((start, actual_end), actual_end)))
            }
        });

        Box::new(stream)
    }
}
