use envconfig::Envconfig;

use integrationos_domain::{cache::CacheConfig, environment::Environment};
use integrationos_domain::database::DatabaseConfig;
use std::{
    fmt::{Display, Formatter, Result},
    net::SocketAddr,
};

#[derive(Envconfig, Clone)]
pub struct StorageConfig {
    #[envconfig(from = "WORKER_THREADS")]
    pub worker_threads: Option<usize>,
    #[envconfig(from = "INTERNAL_SERVER_ADDRESS", default = "0.0.0.0:3005")]
    pub address: SocketAddr,
    #[envconfig(from = "CACHE_SIZE", default = "100")]
    pub cache_size: u64,
    #[envconfig(nested = true)]
    pub db_config: DatabaseConfig,
    #[envconfig(nested = true)]
    pub cache: CacheConfig,
    #[envconfig(from = "ENVIRONMENT", default = "development")]
    pub environment: Environment,

}

impl Display for StorageConfig {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        writeln!(f, "WORKER_THREADS: {:?}", self.worker_threads)?;
        writeln!(f, "INTERNAL_SERVER_ADDRESS: {}", self.address)?;
        writeln!(f, "CACHE_SIZE: {}", self.cache_size)?;
        writeln!(f, "{}", self.environment)?;
        writeln!(f, "{}", self.cache)?;
        writeln!(f, "{}", self.db_config)
    }
}
