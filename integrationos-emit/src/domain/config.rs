use envconfig::Envconfig;
use integrationos_domain::{
    cache::CacheConfig,
    {database::DatabaseConfig, environment::Environment},
};
use std::{
    fmt::{Display, Formatter},
    net::SocketAddr,
};

#[derive(Envconfig, Clone)] // Intentionally no Debug so secret is not printed
pub struct EmitterConfig {
    #[envconfig(from = "SERVER_ADDRESS", default = "0.0.0.0:3000")]
    pub address: SocketAddr,
    #[envconfig(from = "CACHE_SIZE", default = "10000")]
    pub cache_size: u64,
    #[envconfig(from = "SECRET", default = "32KFFT_i4UpkJmyPwY2TGzgHpxfXs7zS")]
    pub secret_key: String,
    #[envconfig(from = "ENVIRONMENT", default = "live")]
    pub environment: Environment,
    #[envconfig(from = "EVENT_TOPIC", default = "events")]
    pub event_topic: String,
    #[envconfig(from = "FLUVIO_ADMIN_PORT", default = "9003")]
    pub fluvio_admin_port: u16,
    #[envconfig(from = "FLUVIO_HOST", default = "localhost")]
    pub fluvio_host: String,
    #[envconfig(from = "FLUVIO_CONNECTION_TIMEOUT_IN_SECS", default = "30")]
    pub fluvio_connection_timeout_in_secs: u64,
    #[envconfig(nested = true)]
    pub cache: CacheConfig,
    #[envconfig(nested = true)]
    pub db: DatabaseConfig,
}

impl Display for EmitterConfig {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "SERVER_ADDRESS: {}", self.address)?;
        writeln!(f, "CACHE_SIZE: {}", self.cache_size)?;
        writeln!(f, "SECRET: ****")?;
        writeln!(f, "ENVIRONMENT: {}", self.environment)?;
        writeln!(f, "EVENT_TOPIC: {}", self.event_topic)?;
        writeln!(f, "FLUVIO_PORT: {}", self.fluvio_admin_port)?;
        writeln!(f, "FLUVIO_HOST: {}", self.fluvio_host)?;
        writeln!(f, "{}", self.cache)?;
        writeln!(f, "{}", self.db)
    }
}
