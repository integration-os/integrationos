use crate::stream::EventStreamProvider;
use envconfig::Envconfig;
use fluvio::dataplane::types::PartitionId;
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
    #[envconfig(from = "EVENT_MAX_SPAN_FOR_RETRY_SECS", default = "86400")]
    pub event_max_span_for_retry_secs: i64,
    #[envconfig(from = "SCHEDULED_MAX_CONCURRENT_TASKS", default = "10")]
    pub scheduled_max_concurrent_tasks: usize,
    #[envconfig(from = "SCHEDULED_SLEEP_DURATION_IN_MILLIS", default = "1000")]
    pub scheduled_sleep_duration_millis: u64,
    #[envconfig(from = "SCHEDULED_MAX_CHUNK_SIZE", default = "100")]
    pub scheduled_max_chunk_size: usize,
    #[envconfig(from = "PUSHER_MAX_CONCURRENT_TASKS", default = "10")]
    pub pusher_max_concurrent_tasks: usize,
    #[envconfig(from = "PUSHER_SLEEP_DURATION_IN_MILLIS", default = "1000")]
    pub pusher_sleep_duration_millis: u64,
    #[envconfig(from = "PUSHER_MAX_CHUNK_SIZE", default = "100")]
    pub pusher_max_chunk_size: usize,
    #[envconfig(from = "SHUTDOWN_TIMEOUT_MILLIS", default = "20000")]
    pub shutdown_timeout_millis: u64,
    #[envconfig(
        from = "JWT_SECRET",
        default = "2thZ2UiOnsibmFtZSI6IlN0YXJ0dXBsa3NoamRma3NqZGhma3NqZGhma3NqZG5jhYtggfaP9ubmVjdGlvbnMiOjUwMDAwMCwibW9kdWxlcyI6NSwiZW5kcG9pbnRzIjo3b4e05e2-f050-401f-9822-44f43f71753c"
    )]
    pub jwt_secret: String,
    #[envconfig(from = "STATEFUL_SET_POD_NAME")]
    pub stateful_set_pod_name: Option<String>,
    #[envconfig(
        from = "EVENT_CALLBACK_URL",
        default = "http://localhost:3005/v1/event-callbacks"
    )]
    pub event_callback_url: String,
    #[envconfig(nested = true)]
    pub fluvio: EventStreamConfig,
    #[envconfig(nested = true)]
    pub cache: CacheConfig,
    #[envconfig(nested = true)]
    pub db_config: DatabaseConfig,
}

impl EmitterConfig {
    /// Returns the partition id to consume from, beware that this assumes several things:
    /// 1. The pod name is in the format of `topic-partition-id` (for example in a statefulset)
    /// 2. Each pod will now have a 1-1 mapping to a partition
    /// 3. It'll read the same partition for the DLQ and the main topic, which means that the DLQ
    ///    and main topic will have the same amount of partitions.
    ///
    /// ## Warning
    /// This is a very brittle assumption, and should be revisited if we ever have a more complex
    /// setup or until this gets resolved: https://github.com/infinyon/fluvio/issues/760
    pub fn partition(&self) -> Option<PartitionId> {
        let pod_name = self.stateful_set_pod_name.as_ref()?;

        if let Some((_, partition_id)) = pod_name.rsplit_once('-') {
            let partition_id = PartitionId::from_str(partition_id).ok()?;
            Some(partition_id)
        } else {
            None
        }
    }
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
            "EVENT_MAX_SPAN_FOR_RETRY_DAYS: {}",
            self.event_max_span_for_retry_secs
        )?;
        writeln!(
            f,
            "PUSHER_MAX_CONCURRENT_TASKS: {}",
            self.pusher_max_concurrent_tasks
        )?;
        writeln!(
            f,
            "PUSHER_SLEEP_DURATION_IN_MILLIS: {}",
            self.pusher_sleep_duration_millis
        )?;
        writeln!(f, "STATEFUL_SET_POD_NAME: {:?}", self.stateful_set_pod_name)?;
        writeln!(f, "PUSHER_MAX_CHUNK_SIZE: {}", self.pusher_max_chunk_size)?;
        writeln!(f, "JWT_SECRET: ****")?;
        writeln!(f, "EVENT_CALLBACK_URL: {}", self.event_callback_url)?;
        writeln!(
            f,
            "EVENT_PROCESSING_MAX_RETRIES: {}",
            self.event_processing_max_retries
        )?;
        writeln!(f, "SHUTDOWN_TIMEOUT_SECS: {}", self.shutdown_timeout_millis)?;
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
