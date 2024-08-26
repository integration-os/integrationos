use envconfig::Envconfig;
use integrationos_domain::database::DatabaseConfig;
use std::fmt::{Display, Formatter};

#[derive(Envconfig, Clone)]
pub struct ArchiverConfig {
    #[envconfig(nested = true)]
    pub db_config: DatabaseConfig,
    pub event_database_url: String,
    #[envconfig(from = "EVENT_COLLECTION_NAME", default = "clients")]
    pub event_collection_name: String,
    #[envconfig(from = "GS_STORAGE_BUCKET", default = "integrationos-zsk")]
    pub gs_storage_bucket: String,
    #[envconfig(from = "GS_STORAGE_URI", default = "gs://integrationos-zsk")]
    pub gs_storage_uri: String,
    #[envconfig(from = "MAX_RETRIES", default = "3")]
    pub max_retries: u32,
    #[envconfig(from = "READ_BUFFER_SIZE_BYTES", default = "262144")]
    pub read_buffer_size: usize,
    #[envconfig(from = "PROCESSING_CHUNK_TIMEOUT_SECS", default = "30")]
    pub processing_chunk_timeout_secs: u64,
}

impl Display for ArchiverConfig {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "EVENT_DATABASE_URL: {}", self.event_database_url)?;
        writeln!(f, "EVENT_COLLECTION_NAME: {}", self.event_collection_name)?;
        writeln!(f, "GS_STORAGE_BUCKET: {}", self.gs_storage_bucket)?;
        writeln!(f, "GS_STORAGE_URI: {}", self.gs_storage_uri)?;
        writeln!(f, "MAX_RETRIES: {}", self.max_retries)?;
        write!(
            f,
            "PROCESSING_CHUNK_TIMEOUT_SECS: {}",
            self.processing_chunk_timeout_secs
        )?;
        writeln!(f, "READ_BUFFER_SIZE_BYTES: {}", self.read_buffer_size)?;
        write!(f, "{}", self.db_config)
    }
}
