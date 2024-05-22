use envconfig::Envconfig;
use integrationos_domain::{
    cache::CacheConfig as RedisConfig, database::DatabaseConfig, secrets::SecretsConfig,
};
use std::fmt::{Display, Formatter};

#[derive(Envconfig, Clone)] // Intentionally no Debug so secret is not printed
pub struct EventCoreConfig {
    #[envconfig(from = "CACHE_SIZE", default = "10000")]
    pub cache_size: u64,
    #[envconfig(from = "CACHE_TTL_SECS", default = "60")]
    pub cache_ttl_secs: u64,
    #[envconfig(from = "DB_CONNECTION_COUNT", default = "25")]
    pub db_connection_count: u64,
    #[envconfig(from = "FETCH_GOOGLE_AUTH_TOKEN", default = "true")]
    pub fetch_google_auth_token: bool,
    #[envconfig(nested = true)]
    pub secrets_config: SecretsConfig,
    #[envconfig(nested = true)]
    pub redis: RedisConfig,
    #[envconfig(nested = true)]
    pub db: DatabaseConfig,
}

impl Display for EventCoreConfig {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "CACHE_SIZE: {}", self.cache_size)?;
        writeln!(f, "CACHE_TTL_SECS: {}", self.cache_ttl_secs)?;
        writeln!(f, "DB_CONNECTION_COUNT: {}", self.db_connection_count)?;
        writeln!(
            f,
            "FETCH_GOOGLE_AUTH_TOKEN: {}",
            self.fetch_google_auth_token
        )?;
        write!(f, "{}", self.secrets_config)?;
        write!(f, "{}", self.redis)?;
        write!(f, "{}", self.db)
    }
}
