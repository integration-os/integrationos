use super::Storage;
use crate::config::ArchiverConfig;
use crate::storage::Chunk;
use crate::Extension;
use anyhow::{Context, Result};
use chrono::Utc;
use google_cloud_storage::client::{Client as GClient, ClientConfig};
use google_cloud_storage::http::objects::upload::{UploadObjectRequest, UploadType};
use google_cloud_storage::http::objects::Object;
use google_cloud_storage::http::resumable_upload_client::ChunkSize;
use integrationos_domain::Unit;
use reqwest_middleware::ClientBuilder;
use reqwest_retry::{policies::ExponentialBackoff, RetryTransientMiddleware};
use reqwest_tracing::TracingMiddleware;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::fs::File;
use tokio::io::{AsyncBufReadExt, BufReader};

#[derive(Clone)]
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
        suffix: String
    ) -> Result<Unit> {
        upload_file_google(base_path, extension, config, &self.client, suffix).await
    }
}

async fn upload_file_google(
    base_path: &Path,
    extension: &Extension,
    config: &ArchiverConfig,
    storage: &GClient,
    suffix: String
) -> Result<Unit> {
    let path = base_path.with_extension(extension.as_ref());
    let total = path.metadata()?.len();

    let upload_type = UploadType::Multipart(Box::new(Object {
        name: construct_file_name(&path, suffix)?,
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

fn construct_file_name(path: &Path, suffix: String) -> Result<String> {
    let file_name = path
        .file_name()
        .context("Missing file name")?
        .to_str()
        .context("Invalid file name: {path:?}")?;

    let timestamp = Utc::now().format("%Y-%m-%d");
    let file_name = format!("{}-{}-{}", timestamp, suffix, file_name);

    Ok(file_name)
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
        let file_name = construct_file_name(&PathBuf::from(string), "1-2".into()).expect("Failed to get file name");
        let now = Utc::now().format("%Y-%m-%d").to_string();
        assert!(file_name.contains('-'));
        assert!(file_name.contains(now.as_str()));
        assert!(file_name.contains("1-2"));
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
