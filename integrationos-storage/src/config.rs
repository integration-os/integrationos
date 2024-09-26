use anyhow::Result as AnyhowResult;
use envconfig::Envconfig;
use integrationos_domain::{cache::CacheConfig, environment::Environment};
use std::{
    fmt::{Display, Formatter, Result},
    net::SocketAddr,
};
use strum::{AsRefStr, EnumString};
use crate::storage::{PostgresStorage, Storage};

#[derive(Envconfig, Clone)]
pub struct StorageConfig {
    #[envconfig(from = "WORKER_THREADS")]
    pub worker_threads: Option<usize>,
    #[envconfig(from = "INTERNAL_SERVER_ADDRESS", default = "0.0.0.0:3005")]
    pub address: SocketAddr,
    #[envconfig(from = "CACHE_SIZE", default = "100")]
    pub cache_size: u64,
    #[envconfig(nested = true)]
    pub cache: CacheConfig,
    #[envconfig(from = "ENVIRONMENT", default = "development")]
    pub environment: Environment,
    #[envconfig(nested = true)]
    pub postgres_config: PostgresConfig,
    #[envconfig(from = "STORAGE_CONFIG_TYPE", default = "postgres")]
    pub storage_config_type: StorageConfigType,
}

impl Display for StorageConfig {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        writeln!(f, "WORKER_THREADS: {:?}", self.worker_threads)?;
        writeln!(f, "INTERNAL_SERVER_ADDRESS: {}", self.address)?;
        writeln!(f, "CACHE_SIZE: {}", self.cache_size)?;
        writeln!(f, "{}", self.environment)?;
        writeln!(f, "{}", self.cache)?;
        match self.storage_config_type {
            StorageConfigType::Postgres => writeln!(f, "{}", self.postgres_config)
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, EnumString, AsRefStr)]
#[strum(serialize_all = "kebab-case")]
pub enum StorageConfigType {
    Postgres,
}

impl StorageConfigType {
    pub async fn init(&self, configuration: &StorageConfig) -> AnyhowResult<impl Storage> {
        match self {
            StorageConfigType::Postgres => PostgresStorage::new(configuration).await,
        }
    }
}

#[derive(Debug, Clone, Envconfig)]
pub struct PostgresConfig {
    #[envconfig(env = "DATABASE_USER", default = "postgres")]
    pub user: String,
    #[envconfig(env = "DATABASE_PASSWORD", default = "postgres")]
    pub password: String,
    #[envconfig(env = "DATABASE_PORT", default = "5432")]
    pub port: u16,
    #[envconfig(env = "DATABASE_NAME", default = "postgres")]
    pub name: String,
    #[envconfig(env = "DATABASE_HOST", default = "localhost")]
    pub host: String,
    #[envconfig(env = "DATABASE_SSL", default = "false")]
    pub ssl: bool,
    #[envconfig(env = "DATABASE_WAIT_TIMEOUT_IN_MILLIS", default = "1000")]
    pub timeout: u64,
    #[envconfig(env = "DATABASE_POOL_SIZE", default = "10")]
    pub pool_size: u32,
}

impl Display for PostgresConfig {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        writeln!(f, "DATABASE_USER: ****")?;
        writeln!(f, "DATABASE_PASSWORD: ****")?;
        writeln!(f, "DATABASE_PORT: ****")?;
        writeln!(f, "DATABASE_HOST: ****")?;
        writeln!(f, "DATABASE_NAME: {}", self.name)?;
        writeln!(f, "DATABASE_SSL: {}", self.ssl)?;
        writeln!(f, "DATABASE_WAIT_TIMEOUT_IN_MILLIS: {}", self.timeout)?;
        writeln!(f, "DATABASE_POOL_SIZE: {}", self.pool_size)
    }
}
