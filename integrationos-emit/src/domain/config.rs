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
    #[envconfig(from = "INTERNAL_SERVER_ADDRESS", default = "0.0.0.0:3001")]
    pub address: SocketAddr,
    #[envconfig(from = "CACHE_SIZE", default = "10000")]
    pub cache_size: u64,
    #[envconfig(from = "ENVIRONMENT", default = "development")]
    pub environment: Environment,
    #[envconfig(from = "HTTP_CLIENT_TIMEOUT_SECS", default = "30")]
    pub http_client_timeout_secs: u64,
    #[envconfig(from = "HTTP_CLIENT_MAX_RETRIES", default = "3")]
    pub http_client_max_retries: u32,
    #[envconfig(from = "EVENT_STREAM_PROVIDER", default = "logger")]
    pub event_stream_provider: EventStreamProvider,
    #[envconfig(from = "EVENT_PROCESSING_MAX_RETRIES", default = "5")]
    pub event_processing_max_retries: u32,
    #[envconfig(from = "SCHEDULED_MAX_CONCURRENT_TASKS", default = "10")]
    pub scheduled_max_concurrent_tasks: usize,
    #[envconfig(from = "SCHEDULED_SLEEP_DURATION_IN_MILLIS", default = "1000")]
    pub scheduled_sleep_duration_millis: u64,
    #[envconfig(from = "SCHEDULED_MAX_CHUNK_SIZE", default = "100")]
    pub scheduled_max_chunk_size: usize,
    #[envconfig(nested = true)]
    pub fluvio: EventStreamConfig,
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
        writeln!(
            f,
            "HTTP_CLIENT_TIMEOUT_SECS: {}",
            self.http_client_timeout_secs
        )?;
        writeln!(
            f,
            "HTTP_CLIENT_MAX_RETRIES: {}",
            self.http_client_max_retries
        )?;
        writeln!(f, "EVENT_STREAM_PROVIDER: {}", self.event_stream_provider)?;
        writeln!(
            f,
            "EVENT_PROCESSING_MAX_RETRIES: {}",
            self.event_processing_max_retries
        )?;
        writeln!(f, "{}", self.fluvio)?;
        writeln!(f, "{}", self.cache)?;
        writeln!(f, "{}", self.db_config)
    }
}

#[derive(Envconfig, Clone)]
pub struct EventStreamConfig {
    #[envconfig(from = "EVENT_STREAM_HOST", default = "127.0.0.1")]
    pub host: String,
    #[envconfig(from = "EVENT_STREAM_PORT", default = "9003")]
    pub port: u16,
    #[envconfig(from = "EVENT_STREAM_PRODUCER_TOPIC")]
    pub producer_topic: Option<Topic>,
    #[envconfig(from = "EVENT_STREAM_CONSUMER_TOPIC")]
    pub consumer_topic: Option<Topic>,
    #[envconfig(from = "EVENT_STREAM_DLQ_TOPIC", default = "dlq")]
    pub dlq_topic: Topic,
    #[envconfig(from = "EVENT_STREAM_PRODUCER_LINGER_TIME_IN_MILLIS", default = "500")]
    pub producer_linger_time: u64,
    #[envconfig(from = "EVENT_STREAM_PRODUCER_BATCH_SIZE", default = "10000")]
    pub producer_batch_size: usize,
    #[envconfig(
        from = "EVENT_STREAM_CONSUMER_LINGER_TIME_IN_MILLIS",
        default = "10000"
    )]
    pub consumer_linger_time: u64,
    #[envconfig(from = "EVENT_STREAM_CONSUMER_BATCH_SIZE", default = "500")]
    pub consumer_batch_size: usize,
    #[envconfig(from = "EVENT_STREAM_ABSOLUTE_OFFSET")]
    pub absolute_offset: Option<i64>,
    #[envconfig(from = "EVENT_STREAM_CONSUMER_GROUP")]
    pub consumer_group: Option<String>,
}

impl EventStreamConfig {
    pub fn endpoint(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

impl Display for EventStreamConfig {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "EVENT_STREAM_HOST: {}", self.host)?;
        writeln!(f, "EVENT_STREAM_PORT: {}", self.port)?;
        writeln!(f, "EVENT_STREAM_CONSUMER_TOPIC: {:?}", self.consumer_topic)?;
        writeln!(f, "EVENT_STREAM_PRODUCER_TOPIC: {:?}", self.producer_topic)?;
        writeln!(f, "EVENT_STREAM_DLQ_TOPIC: {:?}", self.dlq_topic)?;
        writeln!(
            f,
            "EVENT_STREAM_PRODUCER_LINGER_TIME_IN_MILLIS: {}",
            self.producer_linger_time
        )?;
        writeln!(
            f,
            "EVENT_STREAM_PRODUCER_BATCH_SIZE: {}",
            self.producer_batch_size
        )?;
        writeln!(
            f,
            "EVENT_STREAM_CONSUMER_LINGER_TIME_IN_MILLIS: {}",
            self.consumer_linger_time
        )?;
        writeln!(
            f,
            "EVENT_STREAM_CONSUMER_BATCH_SIZE: {}",
            self.consumer_batch_size
        )?;
        writeln!(
            f,
            "EVENT_STREAM_ABSOLUTE_OFFSET: {:?}",
            self.absolute_offset
        )?;
        writeln!(f, "EVENT_STREAM_CONSUMER_GROUP: {:?}", self.consumer_group)
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
