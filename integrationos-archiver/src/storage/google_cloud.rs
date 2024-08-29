use anyhow::{anyhow, Context, Result};
use chrono::{NaiveDate, Utc};
use futures::StreamExt;
use google_cloud_storage::client::{Client as GClient, ClientConfig};
use google_cloud_storage::http::objects::download::Range;
use google_cloud_storage::http::objects::get::GetObjectRequest;
use google_cloud_storage::http::objects::list::ListObjectsRequest;
use google_cloud_storage::http::objects::upload::{UploadObjectRequest, UploadType};
use google_cloud_storage::http::objects::Object;
use google_cloud_storage::http::resumable_upload_client::ChunkSize;
use integrationos_domain::Unit;
use reqwest_middleware::ClientBuilder;
use reqwest_retry::{policies::ExponentialBackoff, RetryTransientMiddleware};
use reqwest_tracing::TracingMiddleware;
use std::future::Future;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tempfile::Builder;
use tokio::fs::File;
use tokio::io::{AsyncBufReadExt, BufReader};

use crate::config::ArchiverConfig;
use crate::event::Event;
use crate::storage::Chunk;
use crate::Extension;

use super::{ArchiveName, Storage};

pub struct GoogleCloudStorage {
    client: GClient,
}

impl GoogleCloudStorage {
    pub async fn new(config: &ArchiverConfig) -> Result<Self> {
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

        Ok(GoogleCloudStorage { client: storage })
    }
}

impl Storage for GoogleCloudStorage {
    async fn upload_file(
        &self,
        base_path: &Path,
        extension: &Extension,
        config: &ArchiverConfig,
    ) -> Result<Unit> {
        upload_file_google(base_path, extension, config, &self.client).await
    }

    async fn download_file(
        &self,
        config: &ArchiverConfig,
        event: &Event,
        extension: &Extension,
    ) -> Result<PathBuf> {
        download_file_google(config, &self.client, event, extension).await
    }
}

async fn download_file_google(
    config: &ArchiverConfig,
    storage: &GClient,
    event: &Event,
    extension: &Extension,
) -> Result<PathBuf> {
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

    let archive_name = find_latest_archive(&names, config, event, extension)?;

    let mut download = storage
        .download_streamed_object(
            &GetObjectRequest {
                bucket: config.gs_storage_bucket.clone(),
                object: archive_name.name(),
                ..Default::default()
            },
            &Range::default(),
        )
        .await?;

    let archive_file = Builder::new()
        .suffix(&archive_name.extension.with_leading_dot())
        .prefix(&format!(
            "{}.{}-",
            config.event_collection_name,
            archive_name.date.format("%Y-%m-%d")
        ))
        .tempfile()?
        .keep()?; // If the tempfile is dropped, the file will be deleted

    let path = archive_file.1;
    let mut archive_file = archive_file.0;

    // TODO: Eventually we should decide if there's value in also restoring the collection metadata(e.g. indexes)
    while let Some(result) = download.next().await {
        match result {
            Ok(bytes) => {
                archive_file
                    .write_all(&bytes)
                    .context("Failed to write archive")?;

                archive_file
                    .flush()
                    .context("Failed to flush archive metadata")?;
            }
            Err(e) => anyhow::bail!("Error downloading archive: {}", e),
        }
    }

    Ok(path.to_path_buf())
}

async fn upload_file_google(
    base_path: &Path,
    extension: &Extension,
    config: &ArchiverConfig,
    storage: &GClient,
) -> Result<Unit> {
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
) -> Result<Unit>
where
    F: Fn(Chunk) -> Fut + Send,
    Fut: Future<Output = Result<Unit>> + Send,
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

fn parse_archive_name(
    name: &str,
    config: &ArchiverConfig,
    extension: &Extension,
) -> Option<ArchiveName> {
    let expected_suffix = format!(
        "-{}{}",
        config.event_collection_name,
        extension.with_leading_dot()
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
                            extension.with_leading_dot()
                        )
                    {
                        return Some(ArchiveName {
                            date,
                            name: config.event_collection_name.clone(),
                            extension: *extension,
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
    extension: &Extension,
) -> Result<ArchiveName, anyhow::Error> {
    names
        .iter()
        .flat_map(|name| parse_archive_name(name, config, extension))
        .filter(|archive| archive.date == event.date())
        .max_by_key(|archive| archive.date)
        .ok_or_else(|| anyhow!("No valid archive found. Please check the date you are restoring is the same as the event date. Or that there are any archived events for this collection."))
}

#[cfg(test)]
mod tests {
    use crate::event::completed::Completed;

    use super::*;
    use envconfig::Envconfig;
    use fake::{Fake, Faker};
    use integrationos_domain::{prefix::IdPrefix, Id};
    use std::{
        collections::HashMap,
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

    #[test]
    fn test_find_latest_archive() {
        let config = ArchiverConfig::init_from_hashmap(&HashMap::from_iter(vec![
            (
                "EVENT_DATABASE_URL".to_string(),
                "mongodb://localhost:27017".to_string(),
            ),
            ("EVENT_COLLECTION_NAME".to_string(), "clients".to_string()),
        ]))
        .expect("Failed to initialize archiver config");

        let date = Utc::now();

        let completed = Completed::new(
            "events-service/events.bson.gz".to_string(),
            Id::new(IdPrefix::Archive, date),
        )
        .with_date(date);

        let event = Event::Completed(completed);

        let extension = Extension::Bson;

        let archive_name = find_latest_archive(
            &[
                format!(
                    "{}-{}.bson.gz",
                    date.format("%Y-%m-%d"),
                    config.event_collection_name
                ),
                format!(
                    "{}-{}.metadata.json.gz",
                    date.format("%Y-%m-%d"),
                    config.event_collection_name
                ),
            ],
            &config,
            &event,
            &extension,
        )
        .expect("Failed to find latest archive");

        assert_eq!(archive_name.name, "clients".to_string());
        assert_eq!(archive_name.extension, Extension::Bson);
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
