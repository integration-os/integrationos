use envconfig::Envconfig;
use integrationos_domain::{cache::CacheConfig, database::DatabaseConfig, secrets::SecretsConfig};
use std::fmt::{Display, Formatter};

#[derive(Envconfig, Clone)] // Intentionally no Debug so secret is not printed
pub struct SnapshotConfig {
    #[envconfig(from = "CACHE_SIZE", default = "10000")]
    pub cache_size: u64,
    #[envconfig(from = "CACHE_TTL_SECS", default = "60")]
    pub cache_ttl_secs: u64,
    #[envconfig(nested = true)]
    pub secrets_config: SecretsConfig,
    #[envconfig(nested = true)]
    pub cache: CacheConfig,
    #[envconfig(nested = true)]
    pub db: DatabaseConfig,
    #[envconfig(from = "STREAM_CHUNK_SIZE", default = "100")]
    pub stream_chunk_size: usize,
    #[envconfig(from = "STREAM_CONCURRENCY", default = "10")]
    pub stream_concurrency: usize,
}

impl Display for SnapshotConfig {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "CACHE_SIZE: {}", self.cache_size)?;
        writeln!(f, "CACHE_TTL_SECS: {}", self.cache_ttl_secs)?;
        writeln!(f, "STREAM_CHUNK_SIZE: {}", self.stream_chunk_size)?;
        writeln!(f, "STREAM_CONCURRENCY: {}", self.stream_concurrency)?;
        write!(f, "{}", self.secrets_config)?;
        write!(f, "{}", self.cache)?;
        write!(f, "{}", self.db)
    }
}
