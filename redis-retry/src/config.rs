use envconfig::Envconfig;
use std::fmt::{Display, Formatter};

#[derive(Envconfig, Debug, Clone)]
pub struct Config {
    #[envconfig(from = "REDIS_URL", default = "redis://localhost:6379")]
    pub url: String,
    #[envconfig(from = "REDIS_QUEUE_NAME", default = "events")]
    pub queue_name: String,
    #[envconfig(from = "REDIS_EVENT_THROUGHPUT_KEY", default = "event_throughput")]
    pub event_throughput_key: String,
    #[envconfig(from = "REDIS_API_THROUGHPUT_KEY", default = "api_throughput")]
    pub api_throughput_key: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            url: "redis://localhost:6379".to_owned(),
            queue_name: "events".to_owned(),
            event_throughput_key: "event_throughput".to_owned(),
            api_throughput_key: "api_throughput".to_owned(),
        }
    }
}

impl Display for Config {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "REDIS_URL: {}", self.url)?;
        writeln!(f, "REDIS_QUEUE_NAME: {}", self.queue_name)?;
        writeln!(
            f,
            "REDIS_EVENT_THROUGHPUT_KEY: {}",
            self.event_throughput_key
        )?;
        writeln!(f, "REDIS_API_THROUGHPUT_KEY: {}", self.api_throughput_key)
    }
}
