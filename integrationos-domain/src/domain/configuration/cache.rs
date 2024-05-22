use envconfig::Envconfig;
use std::fmt::{Display, Formatter};

#[derive(Envconfig, Debug, Clone)]
pub struct CacheConfig {
    #[envconfig(from = "REDIS_URL", default = "redis://localhost:6379")]
    pub url: String,
    #[envconfig(from = "REDIS_QUEUE_NAME", default = "events")]
    pub queue_name: String,
    #[envconfig(from = "REDIS_EVENT_THROUGHPUT_KEY", default = "event_throughput")]
    pub event_throughput_key: String,
    #[envconfig(from = "REDIS_API_THROUGHPUT_KEY", default = "api_throughput")]
    pub api_throughput_key: String,
    #[envconfig(from = "REDIS_POOL_SIZE", default = "10")]
    pub pool_size: usize,
    #[envconfig(env = "CACHE_WAIT_TIMEOUT_SECONDS", default = "30")]
    pub wait_timeout: u64,
    #[envconfig(env = "CACHE_CREATE_TIMEOUT_SECONDS", default = "30")]
    pub create_timeout: u64,
    #[envconfig(env = "CACHE_RECYCLE_TIMEOUT_SECONDS", default = "30")]
    pub recycle_timeout: u64,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            url: "redis://localhost:6379".to_owned(),
            queue_name: "events".to_owned(),
            event_throughput_key: "event_throughput".to_owned(),
            api_throughput_key: "api_throughput".to_owned(),
            pool_size: 10,
            wait_timeout: 30,
            create_timeout: 30,
            recycle_timeout: 30,
        }
    }
}

impl Display for CacheConfig {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "REDIS_URL: {}", self.url)?;
        writeln!(f, "REDIS_QUEUE_NAME: {}", self.queue_name)?;
        writeln!(
            f,
            "REDIS_EVENT_THROUGHPUT_KEY: {}",
            self.event_throughput_key
        )?;
        writeln!(f, "REDIS_API_THROUGHPUT_KEY: {}", self.api_throughput_key)?;
        writeln!(f, "REDIS_POOL_SIZE: {}", self.pool_size)?;
        writeln!(f, "CACHE_WAIT_TIMEOUT_SECONDS: {}", self.wait_timeout)?;
        writeln!(f, "CACHE_CREATE_TIMEOUT_SECONDS: {}", self.create_timeout)?;
        writeln!(f, "CACHE_RECYCLE_TIMEOUT_SECONDS: {}", self.recycle_timeout)
    }
}
