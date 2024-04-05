use envconfig::Envconfig;
use integrationos_domain::common::database::DatabaseConfig;
use redis_retry::Config as RedisConfig;
use std::fmt::{Display, Formatter};

#[derive(Envconfig, Clone)] // Intentionally no Debug so secret is not printed
pub struct Config {
    #[envconfig(from = "EVENT_TIMEOUT", default = "300")] // 300 seconds/ 5 minutes
    pub event_timeout: u64,
    #[envconfig(from = "POLL_DURATION", default = "10")] // 10 seconds
    pub poll_duration: u64,
    #[envconfig(nested = true)]
    pub redis: RedisConfig,
    #[envconfig(nested = true)]
    pub db: DatabaseConfig,
}

impl Display for Config {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "POLL_DURATION: {}", self.poll_duration)?;
        writeln!(f, "EVENT_TIMEOUT: {}", self.event_timeout)?;
        writeln!(f, "{}", self.redis)?;
        writeln!(f, "{}", self.db)
    }
}
