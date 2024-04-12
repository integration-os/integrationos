use envconfig::Envconfig;
use integrationos_domain::{
    cache::CacheConfig,
    common::{database::DatabaseConfig, environment::Environment},
};
use std::{
    fmt::{Display, Formatter},
    net::SocketAddr,
};

#[derive(Envconfig, Clone)] // Intentionally no Debug so secret is not printed
pub struct Config {
    #[envconfig(from = "SERVER_ADDRESS", default = "0.0.0.0:3000")]
    pub address: SocketAddr,
    #[envconfig(from = "CACHE_SIZE", default = "10000")]
    pub cache_size: u64,
    #[envconfig(from = "SECRET", default = "32KFFT_i4UpkJmyPwY2TGzgHpxfXs7zS")]
    pub secret_key: String,
    #[envconfig(from = "ENVIRONMENT", default = "live")]
    pub environment: Environment,
    #[envconfig(nested = true)]
    pub redis: CacheConfig,
    #[envconfig(nested = true)]
    pub db: DatabaseConfig,
}

impl Config {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Display for Config {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "SERVER_ADDRESS: {}", self.address)?;
        writeln!(f, "CACHE_SIZE: {}", self.cache_size)?;
        writeln!(f, "SECRET: ****")?;
        writeln!(f, "ENVIRONMENT: {}", self.environment)?;
        writeln!(f, "{}", self.redis)?;
        writeln!(f, "{}", self.db)
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            address: "0.0.0.0:3000".parse().unwrap(),
            cache_size: 10_000,
            secret_key: "32KFFT_i4UpkJmyPwY2TGzgHpxfXs7zS".to_owned(),
            environment: Environment::Test,
            redis: CacheConfig::default(),
            db: DatabaseConfig::default(),
        }
    }
}

#[cfg(test)]

mod tests {
    use super::*;

    #[test]
    fn test_config() {
        let config = Config::new();
        assert_eq!(config.address, "0.0.0.0:3000".parse().unwrap());
        assert_eq!(config.cache_size, 10_000);
        assert_eq!(config.secret_key, "32KFFT_i4UpkJmyPwY2TGzgHpxfXs7zS");
        assert_eq!(config.environment, Environment::Test);
        assert_eq!(config.redis.url, "redis://localhost:6379");
        assert_eq!(config.redis.queue_name, "events");
        assert_eq!(config.redis.event_throughput_key, "event_throughput");
        assert_eq!(config.db.event_db_url, "mongodb://localhost:27017");
        assert_eq!(config.db.event_db_name, "database");
        assert_eq!(config.db.control_db_url, "mongodb://localhost:27017");
        assert_eq!(config.db.control_db_name, "database");
        assert_eq!(config.db.context_db_url, "mongodb://localhost:27017");
        assert_eq!(config.db.context_db_name, "database");
        assert_eq!(config.db.context_collection_name, "event-transactions");
    }

    #[test]
    fn test_config_display() {
        let config = Config::new();
        let mut display = r"SERVER_ADDRESS: 0.0.0.0:3000
CACHE_SIZE: 10000
SECRET: ****
ENVIRONMENT: test
"
        .to_string();

        display += &config.redis.to_string();
        display += "\n";
        display += &config.db.to_string();
        display += "\n";

        assert_eq!(config.to_string(), display);
    }
}
