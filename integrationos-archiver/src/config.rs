use crate::storage::StorageProvider;
use envconfig::Envconfig;
use integrationos_domain::database::DatabaseConfig;
use std::fmt::{Display, Formatter};
use strum::{AsRefStr, EnumString};

#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumString, AsRefStr)]
#[strum(serialize_all = "kebab-case")]
pub enum Mode {
    Dump,
    DumpDelete,
    NoOp,
}

#[derive(Envconfig, Clone)]
pub struct ArchiverConfig {
    #[envconfig(nested = true)]
    pub db_config: DatabaseConfig,
    #[envconfig(from = "EVENT_COLLECTION_NAME", default = "clients")]
    pub event_collection_name: String,
    #[envconfig(from = "GS_STORAGE_BUCKET", default = "integrationos-zsk")]
    pub gs_storage_bucket: String,
    #[envconfig(from = "GS_STORAGE_URI", default = "gs://integrationos-zsk")]
    pub gs_storage_uri: String,
    #[envconfig(from = "STORAGE_PROVIDER", default = "google-cloud")]
    pub storage_provider: StorageProvider,
    #[envconfig(from = "MAX_RETRIES", default = "3")]
    pub max_retries: u32,
    #[envconfig(from = "READ_BUFFER_SIZE_BYTES", default = "262144")]
    pub read_buffer_size: usize,
    #[envconfig(from = "PROCESSING_CHUNK_TIMEOUT_SECS", default = "30")]
    pub processing_chunk_timeout_secs: u64,
    #[envconfig(from = "MIN_DATE_DAYS", default = "30")]
    pub min_date_days: i64,
    #[envconfig(from = "CHUNK_SIZE_MINUTES", default = "20")]
    pub chunk_size_minutes: i64,
    #[envconfig(from = "CONCURRENT_CHUNKS", default = "10")]
    pub concurrent_chunks: usize,
    #[envconfig(from = "SLEEP_AFTER_FINISH_DUMP_SECS", default = "60")]
    pub sleep_after_finish: u64,
    #[envconfig(from = "MODE", default = "dump")]
    pub mode: Mode,
}

impl Display for ArchiverConfig {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "GS_STORAGE_BUCKET: {}", self.gs_storage_bucket)?;
        writeln!(f, "GS_STORAGE_URI: {}", self.gs_storage_uri)?;
        writeln!(f, "MAX_RETRIES: {}", self.max_retries)?;
        writeln!(f, "STORAGE_PROVIDER: {}", self.storage_provider.as_ref())?;
        writeln!(
            f,
            "PROCESSING_CHUNK_TIMEOUT_SECS: {}",
            self.processing_chunk_timeout_secs
        )?;
        writeln!(f, "READ_BUFFER_SIZE_BYTES: {}", self.read_buffer_size)?;
        writeln!(f, "MIN_DATE_DAYS: {}", self.min_date_days)?;
        writeln!(f, "CHUNK_SIZE_MINUTES: {}", self.chunk_size_minutes)?;
        writeln!(f, "EVENT_COLLECTION_NAME: {}", self.event_collection_name)?;
        writeln!(
            f,
            "SLEEP_AFTER_FINISH_DUMP_SECS: {}",
            self.sleep_after_finish
        )?;
        writeln!(f, "CONCURRENT_CHUNKS: {}", self.concurrent_chunks)?;
        writeln!(f, "MODE: {}", self.mode.as_ref())?;
        write!(f, "{}", self.db_config)
    }
}
