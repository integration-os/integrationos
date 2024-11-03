use crate::stream::EventStreamProvider;
use envconfig::Envconfig;
use integrationos_domain::{
    cache::CacheConfig,
    {database::DatabaseConfig, environment::Environment},
};
use std::{
    fmt::{Display, Formatter},
    net::SocketAddr,
    str::FromStr,
};

#[derive(Envconfig, Clone)] // Intentionally no Debug so secret is not printed
pub struct EmitterConfig {
    #[envconfig(from = "API_VERSION", default = "v1")]
    pub api_version: String,
    #[envconfig(from = "WORKER_THREADS")]
    pub worker_threads: Option<usize>,
    #[envconfig(from = "SERVER_ADDRESS", default = "0.0.0.0:3001")]
    pub address: SocketAddr,
    #[envconfig(from = "CACHE_SIZE", default = "10000")]
    pub cache_size: u64,
    #[envconfig(from = "ENVIRONMENT", default = "live")]
    pub environment: Environment,
    #[envconfig(from = "HTTP_CLIENT_TIMEOUT_SECS", default = "30")]
    pub http_client_timeout_secs: u64,
    #[envconfig(from = "HTTP_CLIENT_MAX_RETRIES", default = "3")]
    pub http_client_max_retries: u32,
    #[envconfig(from = "EVENT_STREAM_PROVIDER", default = "logger")]
    pub event_stream_provider: EventStreamProvider,
    #[envconfig(nested = true)]
    pub fluvio: StreamConfig,
    #[envconfig(nested = true)]
    pub cache: CacheConfig,
    #[envconfig(nested = true)]
    pub db_config: DatabaseConfig,
}

impl Display for EmitterConfig {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "SERVER_ADDRESS: {}", self.address)?;
        writeln!(f, "CACHE_SIZE: {}", self.cache_size)?;
        writeln!(f, "SECRET: ****")?;
        writeln!(f, "ENVIRONMENT: {}", self.environment)?;
        writeln!(f, "{}", self.fluvio)?;
        writeln!(f, "{}", self.cache)?;
        writeln!(f, "{}", self.db_config)
    }
}

#[derive(Envconfig, Clone)]
pub struct StreamConfig {
    #[envconfig(from = "FLUVIO_HOST", default = "127.0.0.1")]
    pub host: String,
    #[envconfig(from = "FLUVIO_PORT", default = "9003")]
    pub port: u16,
    #[envconfig(from = "FLUVIO_PRODUCER_TOPIC")]
    pub producer_topic: Option<Topic>,
    #[envconfig(from = "FLUVIO_CONSUMER_TOPIC")]
    pub consumer_topic: Option<Topic>,
    #[envconfig(from = "FLUVIO_DLQ_TOPIC", default = "dlq")]
    pub dlq_topic: Topic,
    #[envconfig(from = "FLUVIO_LINGER_TIME_IN_MILLIS", default = "500")]
    pub linger_time: u64,
    #[envconfig(from = "FLUVIO_BATCH_SIZE", default = "500")]
    pub batch_size: usize,
    #[envconfig(from = "FLUVIO_CONSUMER_GROUP")]
    pub consumer_group: Option<String>, // Not needed until https://github.com/infinyon/fluvio/issues/760
}

impl StreamConfig {
    pub fn endpoint(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

impl Display for StreamConfig {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "FLUVIO_HOST: {}", self.host)?;
        writeln!(f, "FLUVIO_PORT: {}", self.port)?;
        writeln!(f, "FLUVIO_CONSUMER_TOPIC: {:?}", self.consumer_topic)?;
        writeln!(f, "FLUVIO_PRODUCER_TOPIC: {:?}", self.producer_topic)?;
        writeln!(f, "FLUVIO_DLQ_TOPIC: {:?}", self.dlq_topic)?;
        writeln!(f, "FLUVIO_LINGER_TIME_IN_MILLIS: {}", self.linger_time)?;
        writeln!(f, "FLUVIO_BATCH_SIZE: {}", self.batch_size)?;
        writeln!(f, "FLUVIO_CONSUMER_GROUP: {:?}", self.consumer_group)
    }
}

#[derive(Debug, Clone)]
pub struct Topic(String);

impl<'a> From<&'a Topic> for String {
    fn from(topic: &'a Topic) -> Self {
        topic.0.clone()
    }
}

impl FromStr for Topic {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Topic(s.to_string()))
    }
}

impl Display for Topic {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{}", self.0)
    }
}
