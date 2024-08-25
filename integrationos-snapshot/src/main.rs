mod config;
mod event;

use anyhow::{anyhow, Context, Result};
use bson::doc;
use chrono::{Duration as CDuration, Utc};
use config::SnapshotConfig;
use envconfig::Envconfig;
use event::completed::Completed;
use event::dumped::Dumped;
use event::failed::Failed;
use event::started::Started;
use event::uploaded::Uploaded;
use event::{Event, EventMetadata};
use google_cloud_storage::client::{Client as GClient, ClientConfig};
use google_cloud_storage::http::objects::upload::{UploadObjectRequest, UploadType};
use google_cloud_storage::http::objects::Object;
use google_cloud_storage::http::resumable_upload_client::ChunkSize;
use integrationos_domain::telemetry::{get_subscriber, init_subscriber};
use integrationos_domain::{MongoStore, Store};
use mongodb::Client;
use reqwest_middleware::ClientBuilder;
use reqwest_retry::{policies::ExponentialBackoff, RetryTransientMiddleware};
use reqwest_tracing::TracingMiddleware;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;
use tempfile::TempDir;
use tokio::fs::File;
use tokio::io::{AsyncBufReadExt, BufReader};

#[tokio::main]
async fn main() -> Result<()> {
    let config = SnapshotConfig::init_from_env()?;
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

    let subscriber = get_subscriber("snapshot".into(), "info".into(), std::io::stdout);
    init_subscriber(subscriber);

    tracing::info!("Starting Snapshot with config:\n{config}");

    let client = Client::with_uri_str(&config.db_config.control_db_url).await?;
    let database = client.database(&config.db_config.control_db_name);
    let snapshot: MongoStore<Event> = MongoStore::new(&database, &Store::Snapshots).await?;

    let started = Started::new();
    snapshot
        .create_one(&Event::Started(started.clone()))
        .await?;

    let saved = save(config, &snapshot, storage, &started).await;

    if let Err(e) = saved {
        snapshot
            .create_one(&Event::Failed(Failed::new(
                e.to_string(),
                started.reference(),
            )))
            .await?;

        tracing::error!("Failed to save snapshot: {e}");

        return Err(e);
    }

    tracing::info!("Snapshot saved successfully");

    Ok(())
}

async fn save(
    config: SnapshotConfig,
    snapshot: &MongoStore<Event>,
    storage: GClient,
    started: &Started,
) -> Result<()> {
    let tmp_dir = TempDir::new()?;
    let filter =
        doc! { "createdAt": { "$lt": (Utc::now() - CDuration::days(30)).timestamp_millis() } };

    let command = Command::new("mongodump")
        .arg("--uri")
        .arg(&config.db_config.control_db_url)
        .arg("--db")
        .arg(&config.db_config.control_db_name)
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

    snapshot
        .create_one(&Event::Dumped(Dumped::new(started.reference())))
        .await?;

    let base_path = tmp_dir
        .path()
        .join(&config.db_config.event_db_name)
        .join(&config.event_collection_name);

    if let Err(e) = upload_file(&base_path, "bson.gz", &config, &storage).await {
        return Err(anyhow!("Failed to upload bson file: {e}"));
    }

    if let Err(e) = upload_file(&base_path, "metadata.json.gz", &config, &storage).await {
        return Err(anyhow!("Failed to upload json file: {e}"));
    }

    let remote_path = format!("gs://{}{}", config.gs_storage_bucket, base_path.display());

    snapshot
        .create_one(&Event::Uploaded(Uploaded::new(
            remote_path.clone(),
            started.reference(),
        )))
        .await?;

    tracing::info!("Uploaded files to {}", remote_path);

    snapshot
        .create_one(&Event::Completed(Completed::new(
            remote_path,
            started.reference(),
        )))
        .await?;

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
    extension: &str,
    config: &SnapshotConfig,
    storage: &GClient,
) -> Result<()> {
    let path = base_path.with_extension(extension);
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

// TODO:
// * Restore: mongorestore --gzip --nsInclude=events-service.clients events-service/clients.bson.gz --verbose (nsInclude=${DB_NAME}.${COLLECTION_NAME})
// * Delete: Remove the events that are older than 30 days because they are guaranteed to be already in the snapshot

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
