use super::{cache::CacheConfig, environment::Environment};
use envconfig::Envconfig;
use std::{
    fmt::{Display, Formatter},
    net::SocketAddr,
};
use strum::{AsRefStr, EnumString};

#[derive(Envconfig, Clone)] // Intentionally no Debug so secret is not printed
pub struct DatabaseConfig {
    #[envconfig(from = "CONTROL_DATABASE_URL", default = "mongodb://localhost:27017")]
    pub control_db_url: String,
    #[envconfig(from = "CONTROL_DATABASE_NAME", default = "database")]
    pub control_db_name: String,
    #[envconfig(from = "UDM_DATABASE_URL", default = "mongodb://localhost:27017")]
    pub udm_db_url: String,
    #[envconfig(from = "UDM_DATABASE_NAME", default = "udm")]
    pub udm_db_name: String,
    #[envconfig(from = "EVENT_DATABASE_URL", default = "mongodb://localhost:27017")]
    pub event_db_url: String,
    #[envconfig(from = "EVENT_DATABASE_NAME", default = "database")]
    pub event_db_name: String,
    #[envconfig(from = "CONTEXT_DATABASE_URL", default = "mongodb://localhost:27017")]
    pub context_db_url: String,
    #[envconfig(from = "CONTEXT_DATABASE_NAME", default = "database")]
    pub context_db_name: String,
    #[envconfig(from = "CONTEXT_COLLECTION_NAME", default = "event-transactions")]
    pub context_collection_name: String,
}

impl DatabaseConfig {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            control_db_url: "mongodb://localhost:27017".to_owned(),
            control_db_name: "database".to_owned(),
            udm_db_url: "mongodb://localhost:27017".to_owned(),
            udm_db_name: "udm".to_owned(),
            event_db_url: "mongodb://localhost:27017".to_owned(),
            event_db_name: "database".to_owned(),
            context_db_url: "mongodb://localhost:27017".to_owned(),
            context_db_name: "database".to_owned(),
            context_collection_name: "event-transactions".to_owned(),
        }
    }
}

impl Display for DatabaseConfig {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "CONTROL_DATABASE_URL: ****")?;
        writeln!(f, "CONTROL_DATABASE_NAME: {}", self.control_db_name)?;
        writeln!(f, "UDM_DATABASE_URL: ****")?;
        writeln!(f, "UDM_DATABASE_NAME: {}", self.udm_db_name)?;
        writeln!(f, "EVENT_DATABASE_URL: ****")?;
        writeln!(f, "EVENT_DATABASE_NAME: {}", self.event_db_name)?;
        writeln!(f, "CONTEXT_DATABASE_URL: ****")?;
        writeln!(f, "CONTEXT_DATABASE_NAME: {}", self.context_db_name)?;
        writeln!(
            f,
            "CONTEXT_COLLECTION_NAME: {}",
            self.context_collection_name
        )
    }
}

#[derive(Envconfig, Clone)]
pub struct DatabaseConnectionConfig {
    #[envconfig(from = "WORKER_THREADS")]
    pub worker_threads: Option<usize>,
    #[envconfig(from = "INTERNAL_SERVER_ADDRESS", default = "0.0.0.0:5005")]
    pub address: SocketAddr,
    #[envconfig(from = "CACHE_SIZE", default = "100")]
    pub cache_size: u64,
    #[envconfig(nested = true)]
    pub cache: CacheConfig,
    #[envconfig(from = "ENVIRONMENT", default = "development")]
    pub environment: Environment,
    #[envconfig(nested = true)]
    pub postgres_config: PostgresConfig,
    #[envconfig(from = "DATABASE_CONNECTION_TYPE", default = "postgres")]
    pub database_connection_type: DatabaseConnectionType,
}

impl Display for DatabaseConnectionConfig {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "WORKER_THREADS: {:?}", self.worker_threads)?;
        writeln!(f, "INTERNAL_SERVER_ADDRESS: {}", self.address)?;
        writeln!(f, "CACHE_SIZE: {}", self.cache_size)?;
        writeln!(f, "{}", self.environment)?;
        writeln!(f, "{}", self.cache)?;
        match self.database_connection_type {
            DatabaseConnectionType::Postgres => writeln!(f, "{}", self.postgres_config),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, EnumString, AsRefStr)]
#[strum(serialize_all = "kebab-case")]
pub enum DatabaseConnectionType {
    Postgres,
}

#[derive(Debug, Clone, Envconfig)]
pub struct PostgresConfig {
    #[envconfig(env = "DATABASE_USER", default = "postgres")]
    pub username: String,
    #[envconfig(env = "DATABASE_PASSWORD", default = "postgres")]
    pub password: String,
    #[envconfig(env = "DATABASE_PORT", default = "5432")]
    pub port: u16,
    #[envconfig(env = "DATABASE_NAME", default = "postgres")]
    pub name: String,
    #[envconfig(env = "DATABASE_HOST", default = "localhost")]
    pub host: String,
    #[envconfig(env = "DATABASE_SSL", default = "false")]
    pub ssl: bool,
    #[envconfig(env = "DATABASE_WAIT_TIMEOUT_IN_MILLIS", default = "1000")]
    pub timeout: u64,
    #[envconfig(env = "DATABASE_POOL_SIZE", default = "10")]
    pub pool_size: u32,
}

impl Display for PostgresConfig {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "DATABASE_USER: ****")?;
        writeln!(f, "DATABASE_PASSWORD: ****")?;
        writeln!(f, "DATABASE_PORT: ****")?;
        writeln!(f, "DATABASE_HOST: ****")?;
        writeln!(f, "DATABASE_NAME: {}", self.name)?;
        writeln!(f, "DATABASE_SSL: {}", self.ssl)?;
        writeln!(f, "DATABASE_WAIT_TIMEOUT_IN_MILLIS: {}", self.timeout)?;
        writeln!(f, "DATABASE_POOL_SIZE: {}", self.pool_size)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_config() {
        let config = DatabaseConfig::new();

        assert_eq!(
            config.control_db_url,
            "mongodb://localhost:27017".to_owned()
        );
        assert_eq!(config.control_db_name, "database".to_owned());
        assert_eq!(config.event_db_url, "mongodb://localhost:27017".to_owned());
        assert_eq!(config.event_db_name, "database".to_owned());
        assert_eq!(
            config.context_db_url,
            "mongodb://localhost:27017".to_owned()
        );
        assert_eq!(config.context_db_name, "database".to_owned());
        assert_eq!(
            config.context_collection_name,
            "event-transactions".to_owned()
        );
    }

    #[tokio::test]
    async fn test_config_display() {
        let config = DatabaseConfig::new();

        let config_str = format!("{config}");

        let display = "CONTROL_DATABASE_URL: ****\n\
            CONTROL_DATABASE_NAME: database\n\
            UDM_DATABASE_URL: ****\n\
            UDM_DATABASE_NAME: udm\n\
            EVENT_DATABASE_URL: ****\n\
            EVENT_DATABASE_NAME: database\n\
            CONTEXT_DATABASE_URL: ****\n\
            CONTEXT_DATABASE_NAME: database\n\
            CONTEXT_COLLECTION_NAME: event-transactions\n\
        ";

        assert_eq!(config_str, display);
    }
}
