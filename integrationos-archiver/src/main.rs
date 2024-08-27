mod config;
mod event;

use anyhow::{anyhow, Context, Result};
use bson::{doc, Document};
use chrono::{Duration as CDuration, NaiveDate, Utc};
use config::{ArchiverConfig, Mode};
use envconfig::Envconfig;
use event::completed::Completed;
use event::dumped::Dumped;
use event::failed::Failed;
use event::started::Started;
use event::{Event, EventMetadata};
use futures::StreamExt;
use google_cloud_storage::client::{Client as GClient, ClientConfig};
use google_cloud_storage::http::objects::download::Range;
use google_cloud_storage::http::objects::get::GetObjectRequest;
use google_cloud_storage::http::objects::list::ListObjectsRequest;
use google_cloud_storage::http::objects::upload::{UploadObjectRequest, UploadType};
use google_cloud_storage::http::objects::Object;
use google_cloud_storage::http::resumable_upload_client::ChunkSize;
use integrationos_domain::telemetry::{get_subscriber, init_subscriber};
use integrationos_domain::{MongoStore, Store};
use mongodb::options::FindOneOptions;
use mongodb::{Client, Database};
use reqwest_middleware::ClientBuilder;
use reqwest_retry::{policies::ExponentialBackoff, RetryTransientMiddleware};
use reqwest_tracing::TracingMiddleware;
use std::future::Future;
use std::io::Write;
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;
use tempfile::{Builder, TempDir};
use tokio::fs::File;
use tokio::io::{AsyncBufReadExt, BufReader};

#[tokio::main]
async fn main() -> Result<()> {
    let config = ArchiverConfig::init_from_env()?;
    let retry_policy = ExponentialBackoff::builder().build_with_max_retries(config.max_retries);
    let client = reqwest::Client::default();
    let middleware = ClientBuilder::new(client)
        .with(RetryTransientMiddleware::new_with_policy(retry_policy))
        .with(TracingMiddleware::default())
        .build();
    let storage = GClient::new(
        ClientConfig {
            http: Some(middleware),
            ..Default::default()
        }
        .with_auth()
        .await?,
    );

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
        Mode::Dump => dump(config, &archives, &started, storage).await,
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Extension {
    Bson,
    Metadata,
}

impl Extension {
    /// Returns the file extension for the given extension with the leading dot
    fn with_leading_dot(self) -> String {
        ".".to_owned() + self.as_ref()
    }
}

impl AsRef<str> for Extension {
    fn as_ref(&self) -> &str {
        match self {
            Extension::Bson => "bson.gz",
            Extension::Metadata => "metadata.json.gz",
        }
    }
}

impl Deref for Extension {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}

#[derive(Debug)]
struct ArchiveName {
    date: NaiveDate,
    name: String,
    extension: Extension,
}

impl ArchiveName {
    fn name(&self) -> String {
        format!(
            "{}-{}.{}",
            self.date.format("%Y-%m-%d"),
            self.name,
            self.extension.as_ref()
        )
    }
}

async fn restore(
    config: ArchiverConfig,
    archives: &MongoStore<Event>,
    started: &Started,
    storage: GClient,
) -> Result<()> {
    tracing::info!(
        "Starting archiver in restore mode for the {} collection",
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
        // Should this date should match the date of the archive
        Some(event) => {
            let objects = storage
                .list_objects(&ListObjectsRequest {
                    bucket: config.gs_storage_bucket.clone(),
                    ..Default::default()
                })
                .await?;

            let names = objects
                .items
                .into_iter()
                .flat_map(|object| object.into_iter().map(|o| o.name))
                .collect::<Vec<String>>();

            tracing::info!("Found {:?} objects in the bucket", names);

            let archive_name = find_latest_archive(&names, &config, &event)?;

            let mut bson_download = storage
                .download_streamed_object(
                    &GetObjectRequest {
                        bucket: config.gs_storage_bucket.clone(),
                        object: archive_name.name(),
                        ..Default::default()
                    },
                    &Range::default(),
                )
                .await?;

            let mut archive_bson_file = Builder::new()
                .suffix(&archive_name.extension.with_leading_dot())
                .prefix(&format!(
                    "{}.{}-",
                    config.event_collection_name,
                    archive_name.date.format("%Y-%m-%d")
                ))
                .tempfile()?;

            match bson_download.next().await {
                Some(Ok(bytes)) => archive_bson_file
                    .write_all(&bytes)
                    .context("Failed to write archive")?,
                Some(Err(e)) => anyhow::bail!("Error downloading archive: {}", e),
                None => (),
            };

            let companion_metadata = ArchiveName {
                date: archive_name.date,
                name: config.event_collection_name.clone(),
                extension: Extension::Metadata,
            };

            let mut metadata_download = storage
                .download_streamed_object(
                    &GetObjectRequest {
                        bucket: config.gs_storage_bucket.clone(),
                        object: companion_metadata.name(),
                        ..Default::default()
                    },
                    &Range::default(),
                )
                .await?;

            let mut archive_metadata_file = Builder::new()
                .suffix(&companion_metadata.extension.with_leading_dot())
                .prefix(&format!(
                    "{}.{}-",
                    config.event_collection_name,
                    archive_name.date.format("%Y-%m-%d")
                ))
                .tempfile()?;

            match metadata_download.next().await {
                Some(Ok(bytes)) => archive_metadata_file
                    .write_all(&bytes)
                    .context("Failed to write archive metadata")?,
                Some(Err(e)) => anyhow::bail!("Error downloading archive metadata: {}", e),
                None => (),
            };

            // name of the the two temp files
            tracing::info!("archive_bson_file: {:?}", archive_bson_file);

            // * Restore: mongorestore --gzip --nsInclude=events-service.clients events-service/clients.bson.gz --verbose (nsInclude=${DB_NAME}.${COLLECTION_NAME})
            let output = Command::new("mongorestore")
                .arg("--gzip")
                .arg("--nsInclude")
                .arg(format!(
                    "{}.{}",
                    config.db_config.event_db_name, config.event_collection_name
                ))
                .arg(archive_bson_file.path())
                .arg("--verbose")
                .output()?;

            if !output.status.success() {
                anyhow::bail!(
                    "Archive restore failed with status {}",
                    String::from_utf8_lossy(&output.stderr)
                );
            }

            Ok(())
        }
    }
}

async fn dump(
    config: ArchiverConfig,
    archives: &MongoStore<Event>,
    started: &Started,
    storage: GClient,
) -> Result<()> {
    tracing::info!(
        "Starting archiver in dump mode for the {} collection",
        started.collection()
    );

    let saved = save(config, archives, storage, started).await;

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

    // TODO:
    // * Delete: Remove the events that are older than 30 days because they are guaranteed to be already in the snapshot

    Ok(())
}

async fn save(
    config: ArchiverConfig,
    archive: &MongoStore<Event>,
    storage: GClient,
    started: &Started,
) -> Result<()> {
    let tmp_dir = TempDir::new()?;
    let filter =
        doc! { "createdAt": { "$lt": (Utc::now() - CDuration::days(30)).timestamp_millis() } };

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

    if let Err(e) = upload_file(&base_path, &Extension::Bson, &config, &storage).await {
        return Err(anyhow!("Failed to upload bson file: {e}"));
    }

    if let Err(e) = upload_file(&base_path, &Extension::Metadata, &config, &storage).await {
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

#[derive(Debug)]
struct Chunk {
    data: Vec<u8>,
    first_byte: u64,
    last_byte: u64,
}

impl Chunk {
    fn first_byte(&self) -> u64 {
        self.first_byte
    }

    fn last_byte(&self) -> u64 {
        self.last_byte
    }
}

async fn upload_file(
    base_path: &Path,
    extension: &Extension,
    config: &ArchiverConfig,
    storage: &GClient,
) -> Result<()> {
    let path = base_path.with_extension(extension.as_ref());
    let total = path.metadata()?.len();

    let upload_type = UploadType::Multipart(Box::new(Object {
        name: get_file_name(&path)?,
        ..Default::default()
    }));

    let uploader = storage
        .prepare_resumable_upload(
            &UploadObjectRequest {
                bucket: config.gs_storage_bucket.clone(),
                ..Default::default()
            },
            &upload_type,
        )
        .await?;

    process_file_in_chunks(
        &path,
        config.read_buffer_size,
        Duration::from_secs(config.processing_chunk_timeout_secs),
        |chunk| async {
            let size = ChunkSize::new(chunk.first_byte(), chunk.last_byte(), Some(total));
            uploader.upload_multiple_chunk(chunk.data, &size).await?;
            Ok(())
        },
    )
    .await?;

    Ok(())
}

async fn process_file_in_chunks<F, Fut>(
    file_path: &PathBuf,
    chunk_size: usize,
    timeout: Duration,
    process_chunk: F,
) -> Result<()>
where
    F: Fn(Chunk) -> Fut + Send,
    Fut: Future<Output = Result<()>> + Send,
{
    let file = File::open(file_path).await?;
    let mut buffered_reader = BufReader::with_capacity(chunk_size, file);

    let mut current_position: u64 = 0;

    loop {
        let chunk = buffered_reader.fill_buf().await?;
        let chunk_length = chunk.len();

        if chunk_length == 0 {
            break;
        }

        let first_byte = current_position;
        let last_byte = current_position + chunk_length as u64 - 1;

        let chunk = Chunk {
            data: chunk.to_vec(),
            first_byte,
            last_byte,
        };

        current_position = last_byte + 1;

        tokio::time::timeout(timeout, async { process_chunk(chunk).await }).await??;

        tracing::debug!("Processed chunk of size {}", chunk_length);

        buffered_reader.consume(chunk_length);
    }

    Ok(())
}

fn get_file_name(path: &Path) -> Result<String> {
    let file_name = path
        .file_name()
        .context("Missing file name")?
        .to_str()
        .context("Invalid file name: {path:?}")?;

    let timestamp = Utc::now().format("%Y-%m-%d");
    let file_name = format!("{}-{}", timestamp, file_name);

    Ok(file_name)
}

fn parse_archive_name(name: &str, config: &ArchiverConfig) -> Option<ArchiveName> {
    let expected_suffix = format!(
        "-{}{}",
        config.event_collection_name,
        Extension::Bson.with_leading_dot()
    );

    if let Some(point) = name.rfind(&expected_suffix) {
        if point > 0 && point + expected_suffix.len() <= name.len() {
            let (date_str, file_name) = name.split_at(point);
            match NaiveDate::parse_from_str(date_str, "%Y-%m-%d") {
                Ok(date) => {
                    let file_name = file_name[1..].to_string(); // Skip the leading hyphen
                    if file_name
                        == format!(
                            "{}{}",
                            config.event_collection_name,
                            Extension::Bson.with_leading_dot()
                        )
                    {
                        return Some(ArchiveName {
                            date,
                            name: config.event_collection_name.clone(),
                            extension: Extension::Bson,
                        });
                    }
                }
                Err(e) => tracing::warn!("Invalid date: {}", e),
            }
        } else {
            tracing::warn!("Invalid archive name: {}", name);
        }
    } else {
        tracing::warn!("Expected suffix not found in archive name: {}", name);
    }

    None
}

fn find_latest_archive(
    names: &[String],
    config: &ArchiverConfig,
    event: &Event,
) -> Result<ArchiveName, anyhow::Error> {
    names
        .iter()
        .flat_map(|name| parse_archive_name(name, config))
        .filter(|archive| archive.date == event.date())
        .max_by_key(|archive| archive.date)
        .ok_or_else(|| anyhow!("No valid archive found. Please check the date you are restoring is the same as the event date. Or that there are any archived events for this collection."))
}

#[cfg(test)]
mod tests {
    use super::*;
    use fake::{Fake, Faker};
    use std::{
        io::Write,
        sync::{Arc, Mutex},
    };
    use tempfile::NamedTempFile;

    #[test]
    fn test_get_file_name() {
        let string: String = Faker.fake();
        let file_name = get_file_name(&PathBuf::from(string)).expect("Failed to get file name");
        let now = Utc::now().format("%Y-%m-%d").to_string();
        assert!(file_name.contains('-'));
        assert!(file_name.contains(now.as_str()));
    }

    #[tokio::test]
    async fn test_process_file_in_chunks() {
        let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
        let content = b"abcdefghijklmnopqrstuvwxyz0123456789"; // 36 bytes
        temp_file
            .write_all(content)
            .expect("Failed to write to temp file");

        let path = temp_file.path().to_path_buf(); // Keep the temp file open

        let chunk_size = 10;

        let chunks = Arc::new(Mutex::new(Vec::new()));
        let chunks_ref = Arc::clone(&chunks);

        process_file_in_chunks(&path, chunk_size, Duration::from_secs(30), |chunk| {
            let chunks = Arc::clone(&chunks_ref);
            async move {
                let mut chunks = chunks.lock().expect("Failed to lock chunks");
                chunks.push((chunk.first_byte(), chunk.last_byte(), chunk.data.clone()));
                Ok(())
            }
        })
        .await
        .expect("Failed to process file");

        let chunks = chunks.lock().expect("Failed to lock chunks");
        assert_eq!(chunks.len(), 4);

        assert_eq!(chunks[0].0, 0);
        assert_eq!(chunks[0].1, 9);
        assert_eq!(chunks[0].2, b"abcdefghij".to_vec());

        assert_eq!(chunks[1].0, 10);
        assert_eq!(chunks[1].1, 19);
        assert_eq!(chunks[1].2, b"klmnopqrst".to_vec());

        assert_eq!(chunks[2].0, 20);
        assert_eq!(chunks[2].1, 29);
        assert_eq!(chunks[2].2, b"uvwxyz0123".to_vec());

        assert_eq!(chunks[3].0, 30);
        assert_eq!(chunks[3].1, 35);
        assert_eq!(chunks[3].2, b"456789".to_vec());
    }
}
