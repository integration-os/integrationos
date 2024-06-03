use std::{
    fmt::{Display, Formatter, Result},
    net::SocketAddr,
};

use envconfig::Envconfig;
use integrationos_domain::cache::CacheConfig;
use integrationos_domain::{
    database::DatabaseConfig, openai::OpenAiConfig, secrets::SecretsConfig,
};

#[derive(Envconfig, Clone)]
pub struct Config {
    #[envconfig(from = "WORKER_THREADS")]
    pub worker_threads: Option<usize>,
    #[envconfig(from = "DEBUG_MODE", default = "false")]
    pub debug_mode: bool,
    #[envconfig(from = "INTERNAL_SERVER_ADDRESS", default = "0.0.0.0:3005")]
    pub address: SocketAddr,
    #[envconfig(from = "CACHE_SIZE", default = "100")]
    pub cache_size: u64,
    #[envconfig(from = "ACCESS_KEY_CACHE_TTL_SECS", default = "1800")]
    // Half hour access key ttl by default
    pub access_key_cache_ttl_secs: u64,
    #[envconfig(from = "ACCESS_KEY_WHITELIST_REFRESH_INTERVAL_SECS", default = "60")]
    pub access_key_whitelist_refresh_interval_secs: u64,
    #[envconfig(from = "CONNECTION_CACHE_TTL_SECS", default = "120")]
    pub connection_cache_ttl_secs: u64,
    #[envconfig(from = "ENGINEERING_ACCOUNT_ID", default = "engineering_account")]
    pub engineering_account_id: String,
    #[envconfig(from = "CONNECTION_DEFINITION_CACHE_TTL_SECS", default = "120")]
    pub connection_definition_cache_ttl_secs: u64,
    #[envconfig(from = "CONNECTION_OAUTH_DEFINITION_CACHE_TTL_SECS", default = "120")]
    pub connection_oauth_definition_cache_ttl_secs: u64,
    #[envconfig(from = "CONNECTION_MODEL_SCHEMA_TTL_SECS", default = "3600")]
    pub connection_model_schema_cache_ttl_secs: u64,
    #[envconfig(from = "CONNECTION_MODEL_DEFINITION_CACHE_TTL_SECS", default = "3600")]
    pub connection_model_definition_cache_ttl_secs: u64,
    #[envconfig(from = "SECRET_CACHE_TTL_SECS", default = "3600")]
    pub secret_cache_ttl_secs: u64,
    #[envconfig(
        from = "EVENT_ACCESS_PASSWORD",
        default = "32KFFT_i4UpkJmyPwY2TGzgHpxfXs7zS"
    )]
    pub event_access_password: String,
    #[envconfig(from = "EVENT_ACCESS_THROUGHPUT", default = "500")]
    pub event_access_throughput: u64,
    #[envconfig(from = "EVENT_SAVE_BUFFER_SIZE", default = "2048")]
    pub event_save_buffer_size: usize,
    #[envconfig(from = "EVENT_SAVE_TIMEOUT_SECS", default = "30")]
    pub event_save_timeout_secs: u64,
    #[envconfig(from = "METRIC_SAVE_CHANNEL_SIZE", default = "2048")]
    pub metric_save_channel_size: usize,
    #[envconfig(from = "METRIC_SYSTEM_ID", default = "IntegrationOS-Internal-System")]
    pub metric_system_id: String,
    #[envconfig(from = "SEGMENT_WRITE_KEY")]
    pub segment_write_key: Option<String>,
    // In the future, we will want to emit events for internal API actions
    #[envconfig(from = "EMIT_URL", default = "http://127.0.0.1:3000/emit/")]
    pub emit_url: String,
    #[envconfig(nested = true)]
    pub secrets_config: SecretsConfig,
    #[envconfig(
        from = "JWT_SECRET",
        default = "2thZ2UiOnsibmFtZSI6IlN0YXJ0dXBsa3NoamRma3NqZGhma3NqZGhma3NqZG5jhYtggfaP9ubmVjdGlvbnMiOjUwMDAwMCwibW9kdWxlcyI6NSwiZW5kcG9pbnRzIjo3b4e05e2-f050-401f-9822-44f43f71753c"
    )]
    pub jwt_secret: String,
    #[envconfig(from = "BURST_RATE_LIMIT", default = "1")]
    pub burst_rate_limit: u64,
    /// Burst size limit
    #[envconfig(from = "BURST_SIZE_LIMIT", default = "30")]
    pub burst_size: u32,
    #[envconfig(from = "API_VERSION", default = "v1")]
    pub api_version: String,
    #[envconfig(from = "MOCK_LLM", default = "false")]
    pub mock_llm: bool,
    #[envconfig(from = "HTTP_CLIENT_TIMEOUT_SECS", default = "30")]
    pub http_client_timeout_secs: u64,
    #[envconfig(nested = true)]
    pub headers: Headers,
    #[envconfig(nested = true)]
    pub db_config: DatabaseConfig,
    #[envconfig(nested = true)]
    pub openai_config: OpenAiConfig,
    #[envconfig(nested = true)]
    pub cache_config: CacheConfig,
    #[envconfig(from = "RATE_LIMIT_ENABLED", default = "true")]
    pub rate_limit_enabled: bool,
}

impl Display for Config {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        writeln!(f, "WORKER_THREADS: {:?}", self.worker_threads)?;
        writeln!(f, "DEBUG_MODE: {:?}", self.debug_mode)?;
        writeln!(f, "INTERNAL_SERVER_ADDRESS: {}", self.address)?;
        writeln!(f, "CACHE_SIZE: {}", self.cache_size)?;
        writeln!(
            f,
            "ACCESS_KEY_CACHE_TTL_SECS: {}",
            self.access_key_cache_ttl_secs
        )?;
        writeln!(
            f,
            "ACCESS_KEY_WHITELIST_REFRESH_INTERVAL_SECS: {}",
            self.access_key_whitelist_refresh_interval_secs
        )?;
        writeln!(f, "EVENT_ACCESS_PASSWORD: ***")?;
        writeln!(
            f,
            "EVENT_ACCESS_THROUGHPUT: {}",
            self.event_access_throughput
        )?;
        writeln!(f, "EVENT_SAVE_BUFFER_SIZE: {}", self.event_save_buffer_size)?;
        writeln!(
            f,
            "CONNECTION_CACHE_TTL_SECS: {}",
            self.connection_cache_ttl_secs
        )?;
        writeln!(
            f,
            "CONNECTION_DEFINITION_CACHE_TTL_SECS: {}",
            self.connection_definition_cache_ttl_secs
        )?;
        writeln!(
            f,
            "CONNECTION_OAUTH_DEFINITION_CACHE_TTL_SECS: {}",
            self.connection_oauth_definition_cache_ttl_secs
        )?;
        writeln!(
            f,
            "EVENT_SAVE_TIMEOUT_SECS: {}",
            self.event_save_timeout_secs
        )?;
        writeln!(
            f,
            "METRIC_SAVE_CHANNEL_SIZE: {}",
            self.metric_save_channel_size
        )?;
        writeln!(f, "METRIC_SYSTEM_ID: {}", self.metric_system_id)?;
        writeln!(f, "SEGMENT_WRITE_KEY: ***")?;
        writeln!(f, "EMIT_URL: {}", self.emit_url)?;
        writeln!(f, "JWT_SECRET: ***")?;
        write!(f, "{}", self.secrets_config)?;
        writeln!(f, "API_VERSION: {}", self.api_version)?;
        writeln!(f, "MOCK_LLM: {}", self.mock_llm)?;
        writeln!(f, "{}", self.headers)?;
        writeln!(f, "{}", self.db_config)?;
        writeln!(f, "{}", self.openai_config)?;
        writeln!(f, "{}", self.cache_config)?;
        writeln!(f, "RATE_LIMIT_ENABLED: {}", self.rate_limit_enabled)
    }
}

#[derive(Envconfig, Default, Clone)]
pub struct Headers {
    #[envconfig(from = "HEADER_AUTH", default = "x-integrationos-secret")]
    pub auth_header: String,
    #[envconfig(from = "HEADER_CONNECTION", default = "x-integrationos-connection-key")]
    pub connection_header: String,
    #[envconfig(from = "HEADER_CUSTOM_MAP", default = "x-integrationos-custom-map")]
    pub custom_map_header: String,
    #[envconfig(
        from = "HEADER_ENABLE_PASSTHROUGH",
        default = "x-integrationos-enable-passthrough"
    )]
    pub enable_passthrough_header: String,
    #[envconfig(
        from = "HEADER_INCLUDE_OVERFLOW",
        default = "x-integrationos-include-overflow"
    )]
    pub include_overflow_header: String,
    #[envconfig(
        from = "HEADER_DYNAMIC_PLATFORM",
        default = "x-integrationos-dynamic-platform"
    )]
    pub dynamic_platform_header: String,
    #[envconfig(
        from = "HEADER_RATE_LIMIT_LIMIT",
        default = "x-integrationos-rate-limit-limit"
    )]
    pub rate_limit_limit: String,
    #[envconfig(
        from = "HEADER_RATE_LIMIT_REMAINING",
        default = "x-integrationos-rate-limit-remainings"
    )]
    pub rate_limit_remaining: String,
    #[envconfig(
        from = "HEADER_RATE_LIMIT_REST",
        default = "x-integrationos-rate-limit-reset"
    )]
    pub rate_limit_reset: String,
}

impl Headers {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Display for Headers {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        writeln!(f, "HEADER_AUTH: {}", self.auth_header)?;
        writeln!(f, "HEADER_CONNECTION: {}", self.connection_header)?;
        writeln!(f, "HEADER_CUSTOM_MAP: {}", self.custom_map_header)?;
        writeln!(
            f,
            "HEADER_INCLUDE_PASSTHROUGH: {}",
            self.enable_passthrough_header
        )?;
        writeln!(
            f,
            "HEADER_INCLUDE_OVERFLOW: {}",
            self.include_overflow_header
        )?;
        writeln!(
            f,
            "HEADER_DYNAMIC_PLATFORM: {}",
            self.dynamic_platform_header
        )?;
        writeln!(f, "HEADER_RATE_LIMIT_LIMIT: {}", self.rate_limit_limit)?;
        writeln!(
            f,
            "HEADER_RATE_LIMIT_REMAINING: {}",
            self.rate_limit_remaining
        )?;
        writeln!(f, "HEADER_RATE_LIMIT_RESET: {}", self.rate_limit_reset)
    }
}
