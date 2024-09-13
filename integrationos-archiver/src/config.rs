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
    Restore,
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
        writeln!(f, "EVENT_COLLECTION_NAME: {}", self.event_collection_name)?;
        writeln!(f, "MODE: {}", self.mode.as_ref())?;
        write!(f, "{}", self.db_config)
    }
}
