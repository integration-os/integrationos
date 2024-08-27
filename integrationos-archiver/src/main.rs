mod config;
mod event;
mod storage;

use anyhow::{anyhow, Result};
use bson::{doc, Document};
use chrono::{DateTime, Duration as CDuration, Utc};
use config::{ArchiverConfig, Mode};
use envconfig::Envconfig;
use event::completed::Completed;
use event::dumped::Dumped;
use event::failed::Failed;
use event::started::Started;
use event::uploaded::Uploaded;
use event::{Event, EventMetadata};
use integrationos_domain::telemetry::{get_subscriber, init_subscriber};
use integrationos_domain::{MongoStore, Store, Unit};
use mongodb::options::FindOneOptions;
use mongodb::{Client, Database};
use std::process::Command;
use storage::google_cloud::GoogleCloudStorage;
use storage::{Extension, Storage};
use tempfile::TempDir;

#[tokio::main]
async fn main() -> Result<Unit> {
    let config = ArchiverConfig::init_from_env()?;
    let storage = GoogleCloudStorage::new(&config).await?;

    let subscriber = get_subscriber("archiver".into(), "info".into(), std::io::stdout);
    init_subscriber(subscriber);

    tracing::info!("Starting archiver with config:\n{config}");

    let client = Client::with_uri_str(&config.db_config.event_db_url).await?;
    let database = client.database(&config.db_config.event_db_name);
    let archives: MongoStore<Event> = MongoStore::new(&database, &Store::Archives).await?;

    let started = Started::new(config.event_collection_name.clone())?;
    archives
        .create_one(&Event::Started(started.clone()))
        .await?;

    match config.mode {
        Mode::Restore => restore(config, &archives, &started, storage).await,
        Mode::Dump => dump(config, &archives, &started, storage, database, false).await,
        Mode::DumpDelete => dump(config, &archives, &started, storage, database, true).await,
    }
}

async fn restore(
    config: ArchiverConfig,
    archives: &MongoStore<Event>,
    started: &Started,
    storage: impl Storage,
) -> Result<Unit> {
    tracing::info!(
        "Starting archiver in restore mode for the {} collection. Events will not be restored to the original collection, rather a new collection will be created",
        started.collection()
    );

    let filter = doc! {
        "completedAt": {
            "$exists": true,
        }
    };
    let options = FindOneOptions::builder()
        .sort(doc! { "completedAt": -1 })
        .build();
    let archive = archives.collection.find_one(filter, options).await?;

    match archive {
        None => Err(anyhow!(
            "No archive found for the collection {}",
            started.collection()
        )),
        Some(event) => {
            let archive_bson_file_path = storage
                .download_file(&config, &event, &Extension::Bson)
                .await?;

            // * Restore: mongorestore --gzip --nsInclude=events-service.clients events-service/clients.bson.gz --verbose (nsInclude=${DB_NAME}.${COLLECTION_NAME})
            tracing::info!("Restoring collection {}", config.event_collection_name);
            let output = Command::new("mongorestore")
                .arg("--gzip")
                .arg("--nsInclude")
                .arg(format!(
                    "{}.{}",
                    config.db_config.event_db_name, config.event_collection_name
                ))
                .arg(archive_bson_file_path)
                .arg("--verbose")
                .output()?;

            if !output.status.success() {
                anyhow::bail!(
                    "Archive restore failed with status {}",
                    String::from_utf8_lossy(&output.stderr)
                );
            }

            tracing::info!(
                "Collection {} restored successfully",
                config.event_collection_name
            );

            Ok(())
        }
    }
}

async fn dump(
    config: ArchiverConfig,
    archives: &MongoStore<Event>,
    started: &Started,
    storage: impl Storage,
    database: Database,
    destructive: bool,
) -> Result<Unit> {
    tracing::info!(
        "Starting archiver in dump mode for the {} collection",
        started.collection()
    );

    let date = Utc::now() - CDuration::days(30);
    let saved = save(config, archives, storage, started, &date).await;

    if let Err(e) = saved {
        archives
            .create_one(&Event::Failed(Failed::new(
                e.to_string(),
                started.reference(),
            )))
            .await?;

        tracing::error!("Failed to save archive: {e}");

        return Err(e);
    }

    tracing::info!("Archive saved successfully");

    if destructive {
        tracing::warn!("Deleting old events as destructive mode is enabled");
        let store: MongoStore<Document> = MongoStore::new(&database, started.collection()).await?;

        let filter = doc! {
            "createdAt": { "$lt": date.timestamp_millis() }
        };

        store.collection.delete_many(filter, None).await?;
        tracing::warn!("Old events deleted successfully");
    }

    Ok(())
}

async fn save(
    config: ArchiverConfig,
    archive: &MongoStore<Event>,
    storage: impl Storage,
    started: &Started,
    date: &DateTime<Utc>,
) -> Result<Unit> {
    let tmp_dir = TempDir::new()?;
    let filter = doc! { "createdAt": { "$lt": date.timestamp_millis() } };

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
        .create_one(&Event::Dumped(Dumped::new(started.reference())))
        .await?;

    let base_path = tmp_dir
        .path()
        .join(&config.db_config.event_db_name)
        .join(&config.event_collection_name);

    if let Err(e) = storage
        .upload_file(&base_path, &Extension::Bson, &config)
        .await
    {
        return Err(anyhow!("Failed to upload bson file: {e}"));
    }

    archive
        .create_one(&Event::Uploaded(Uploaded::new(started.reference())))
        .await?;

    if let Err(e) = storage
        .upload_file(&base_path, &Extension::Metadata, &config)
        .await
    {
        return Err(anyhow!("Failed to upload json file: {e}"));
    }

    let remote_path = format!("gs://{}{}", config.gs_storage_bucket, base_path.display());

    archive
        .create_one(&Event::Completed(Completed::new(
            remote_path.clone(),
            started.reference(),
        )))
        .await?;

    tracing::info!(
        "Archive completed at {}, saved to {} with reference {}",
        Utc::now(),
        remote_path,
        started.reference()
    );

    Ok(())
}
