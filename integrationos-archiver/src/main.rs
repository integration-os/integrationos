mod domain;
mod event;
mod storage;

use crate::domain::config::{ArchiverConfig, Mode};
use crate::event::finished::Finished;
use anyhow::{anyhow, Result};
use bson::{doc, Document};
use chrono::offset::LocalResult;
use chrono::{DateTime, Duration as CDuration, TimeZone, Utc};
use dotenvy::dotenv;
use envconfig::Envconfig;
use event::chosen::DateChosen;
use event::completed::Completed;
use event::dumped::Dumped;
use event::failed::Failed;
use event::started::Started;
use event::uploaded::Uploaded;
use event::{Event, EventMetadata};
use futures::future::ready;
use futures::stream::{self, Stream};
use futures::{StreamExt, TryStreamExt};
use integrationos_domain::telemetry::{get_subscriber, init_subscriber};
use integrationos_domain::{MongoStore, Store, Unit};
use mongodb::options::FindOneOptions;
use mongodb::Client;
use std::process::Command;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use storage::google_cloud::GoogleCloudStorage;
use storage::{Extension, Storage, StorageProvider};
use tempfile::TempDir;

#[tokio::main]
async fn main() -> Result<Unit> {
    dotenv().ok();
    let config = Arc::new(ArchiverConfig::init_from_env()?);
    let storage = Arc::new(match config.storage_provider {
        StorageProvider::GoogleCloud => GoogleCloudStorage::new(&config).await?,
    });

    let subscriber = get_subscriber("archiver".into(), "info".into(), std::io::stdout, None);
    init_subscriber(subscriber);

    tracing::info!("Starting archiver with config:\n{config}");

    let client = Arc::new(Client::with_uri_str(&config.db_config.event_db_url).await?);
    let database = Arc::new(client.database(&config.db_config.event_db_name));
    let archives: Arc<MongoStore<Event>> =
        Arc::new(MongoStore::new(&database, &Store::Archives).await?);

    let store = Store::from_str(&config.event_collection_name).map_err(|e| anyhow::anyhow!(e))?;
    let target_store: Arc<MongoStore<Document>> =
        Arc::new(MongoStore::new(&database, &store).await?);

    loop {
        let started = Started::new(config.event_collection_name.clone());
        archives
            .create_one(&Event::Started(started.clone()))
            .await?;

        let res = match config.mode {
            Mode::Dump => dump(&config, &archives, &started, &storage, &target_store, false).await,
            Mode::DumpDelete => {
                dump(&config, &archives, &started, &storage, &target_store, true).await
            }
            Mode::NoOp => Ok(()),
        }
        .inspect_err(|e| {
            tracing::error!("Error in archiver: {e}");
        });

        match res {
            Ok(_) => {
                archives
                    .create_one(&Event::Finished(Finished::new(started.reference())))
                    .await?;
            }
            Err(e) => {
                archives
                    .create_one(&Event::Failed(Failed::new(
                        e.to_string(),
                        started.reference(),
                        started.started_at(),
                        Utc::now(),
                    )))
                    .await?;
            }
        };

        tracing::info!("Sleeping for {} seconds", config.sleep_after_finish);
        tokio::time::sleep(Duration::from_secs(config.sleep_after_finish)).await;
    }
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
                    .sort(doc! { "createdAt": 1 }) // Sort by `createdAt` in ascending order
                    .projection(doc! { "createdAt": 1 }) // Only retrieve the `createdAt` field
                    .build(),
            ),
        )
        .await
        .map_err(|e| anyhow!("Failed to find first event in collection: {e}"))?;

    let start = match document {
        Some(document) => document
            .get_i64("createdAt")
            .map_err(|e| anyhow!("Failed to get createdAt from document: {e}"))?,

        None => {
            tracing::info!(
                "No events found in collection {}",
                target_store.collection.name()
            );

            return Ok(());
        }
    };

    let last_chosen_date_event = archives
        .collection
        .find_one(
            doc! {
                "type": "DateChosen"
            },
            FindOneOptions::builder()
                .sort(doc! { "endsAt": -1 })
                .build(),
        )
        .await?;

    tracing::info!("Last chosen date event: {:?}", last_chosen_date_event);

    let started_at = match last_chosen_date_event {
        Some(event) => match event {
            Event::DateChosen(e) => {
                let finished = archives
                    .collection
                    .find_one(
                        doc! {
                            "type": "Finished",
                            "reference": e.reference().to_string()
                        },
                        None,
                    )
                    .await?
                    .map(|e| e.is_finished())
                    .unwrap_or(false);

                tracing::info!("Date chosen event is finished: {}", finished);

                if finished {
                    e.event_date()
                } else {
                    0
                }
            }
            _ => return Err(anyhow!("Invalid event type, DateChosen expected")),
        },
        _ => 0,
    };

    let start = match Utc.timestamp_millis_opt(start.max(started_at)) {
        LocalResult::Single(date) => date,
        _ => return Err(anyhow!("Invalid timestamp")),
    };

    // End date should be a chunk of size config.chunk_to_process_in_days and not bigger than 30 days ago
    let max_end = Utc::now() - CDuration::days(config.min_date_days);
    let end = (start + CDuration::days(config.chunk_to_process_in_days)).min(max_end);

    if start.timestamp_millis() >= end.timestamp_millis() {
        // If the very first event is after the end time, exit
        tracing::warn!("No events to process, exiting");
        return Ok(());
    }

    archives
        .create_one(&Event::DateChosen(DateChosen::new(
            started.reference(),
            start.timestamp_millis(),
            end.timestamp_millis(),
        )))
        .await?;

    tracing::info!("Start date: {}, End date: {}", start, end);

    let chunks = start.divide_by_stream(CDuration::minutes(config.chunk_size_minutes), end); // Chunk size is 20 minutes by default

    let stream = chunks
        .enumerate()
        .map(|(index, (start_time, end_time))| async move {
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
                (&start_time, &end_time),
                index,
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
                    tracing::error!("Failed to save archive: {e}");

                    return Err(e);
                }
            };

            if destructive {
                tracing::warn!("Deleting old events as destructive mode is enabled");
                let filter = doc! {
                    "createdAt": {
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
        // If we return an error it'll prevent from moving forward in the loop
        // return Err(anyhow!("Encountered {} errors during processing", errors.len()));
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
    times: (&DateTime<Utc>, &DateTime<Utc>),
    part: usize,
) -> Result<u64> {
    let (start_time, end_time) = times;
    let tmp_dir = TempDir::new()?;
    let filter = doc! {
        "createdAt": {
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

    // Run this only on debug mode
    if cfg!(debug_assertions) {
        let events = target_store.collection.find(filter.clone(), None).await?;

        let events = events.try_collect::<Vec<_>>().await?;

        let mem_size = std::mem::size_of::<Vec<Document>>()
            + events.capacity() * std::mem::size_of::<Document>();

        tracing::info!("Total size of all the events is {}", mem_size);
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
        .create_one(&Event::Dumped(Dumped::new(
            started_event.reference(),
            *start_time,
            *end_time,
        )))
        .await?;

    let base_path = tmp_dir
        .path()
        .join(&config.db_config.event_db_name)
        .join(&config.event_collection_name);

    let suffix = format!("{}-part-{}", start_time.timestamp_millis(), part);

    if let Err(e) = storage
        .upload_file(&base_path, &Extension::Bson, config, suffix.clone())
        .await
    {
        return Err(anyhow!("Failed to upload bson file: {e}"));
    }

    archive
        .create_one(&Event::Uploaded(Uploaded::new(
            started_event.reference(),
            *start_time,
            *end_time,
        )))
        .await?;

    let name = storage
        .upload_file(&base_path, &Extension::Metadata, config, suffix.clone())
        .await?;

    let remote_path = format!("gs://{}/{}", config.gs_storage_bucket, name);

    archive
        .create_one(&Event::Completed(Completed::new(
            remote_path.clone(),
            started_event.reference(),
            *start_time,
            *end_time,
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
