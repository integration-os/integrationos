use super::environment::Environment;
use envconfig::Envconfig;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
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

    pub fn as_hashmap(&self) -> HashMap<String, String> {
        let mut map = HashMap::new();

        map.insert(
            "CONTROL_DATABASE_URL".to_string(),
            self.control_db_url.to_string(),
        );
        map.insert(
            "CONTROL_DATABASE_NAME".to_string(),
            self.control_db_name.to_string(),
        );
        map.insert("UDM_DATABASE_URL".to_string(), self.udm_db_url.to_string());
        map.insert(
            "UDM_DATABASE_NAME".to_string(),
            self.udm_db_name.to_string(),
        );
        map.insert(
            "EVENT_DATABASE_URL".to_string(),
            self.event_db_url.to_string(),
        );
        map.insert(
            "EVENT_DATABASE_NAME".to_string(),
            self.event_db_name.to_string(),
        );
        map.insert(
            "CONTEXT_DATABASE_URL".to_string(),
            self.context_db_url.to_string(),
        );
        map.insert(
            "CONTEXT_DATABASE_NAME".to_string(),
            self.context_db_name.to_string(),
        );
        map.insert(
            "CONTEXT_COLLECTION_NAME".to_string(),
            self.context_collection_name.to_string(),
        );

        map
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

#[derive(Envconfig, Clone, Serialize, Deserialize)]
pub struct DatabasePodConfig {
    #[envconfig(from = "WORKER_THREADS")]
    pub worker_threads: Option<usize>,
    #[envconfig(from = "INTERNAL_SERVER_ADDRESS", default = "0.0.0.0:5005")]
    pub address: SocketAddr,
    #[envconfig(from = "ENVIRONMENT", default = "development")]
    pub environment: Environment,
    #[envconfig(from = "EMIT_URL", default = "http://localhost:3001")]
    pub emit_url: String,
    #[envconfig(from = "EMITTER_ENABLED", default = "false")]
    pub emitter_enabled: bool,
    #[envconfig(from = "CONNECTIONS_URL", default = "http://localhost:3005")]
    pub connections_url: String,
    #[envconfig(from = "DATABASE_CONNECTION_TYPE", default = "postgresql")]
    pub database_connection_type: DatabaseConnectionType,
    #[envconfig(from = "CONNECTION_ID")]
    pub connection_id: String,
    #[envconfig(from = "JWT_SECRET")]
    pub jwt_secret: Option<String>,
}

impl DatabasePodConfig {
    pub fn as_hashmap(&self) -> HashMap<String, String> {
        let mut map = HashMap::new();

        map.insert(
            "WORKER_THREADS".to_string(),
            self.worker_threads.unwrap_or(1).to_string(),
        );
        map.insert(
            "INTERNAL_SERVER_ADDRESS".to_string(),
            self.address.to_string(),
        );
        map.insert(
            "EMITTER_ENABLED".to_string(),
            self.emitter_enabled.to_string(),
        );
        map.insert("ENVIRONMENT".to_string(), self.environment.to_string());
        map.insert(
            "DATABASE_CONNECTION_TYPE".to_string(),
            self.database_connection_type.as_ref().into(),
        );
        map.insert("EMIT_URL".to_string(), self.emit_url.clone());
        map.insert("CONNECTION_ID".to_string(), self.connection_id.clone());
        map.insert(
            "DATABASE_CONNECTION_TYPE".to_string(),
            self.database_connection_type.as_ref().into(),
        );

        map
    }
}

impl Display for DatabasePodConfig {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "WORKER_THREADS: {:?}", self.worker_threads)?;
        writeln!(f, "INTERNAL_SERVER_ADDRESS: {}", self.address)?;
        writeln!(f, "EMIT_URL: {}", self.emit_url)?;
        writeln!(f, "ENVIRONMENT: {}", self.environment)?;
        writeln!(f, "EMITTER_ENABLED: {}", self.emitter_enabled)?;
        writeln!(f, "JWT_SECRET: ***")?;
        writeln!(
            f,
            "DATABASE_CONNECTION_TYPE: {:?}",
            self.database_connection_type.as_ref()
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, EnumString, AsRefStr, Serialize, Deserialize)]
#[strum(serialize_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum DatabaseConnectionType {
    PostgreSql,
}

#[derive(Debug, Clone, Envconfig, Default, Serialize, Deserialize, PartialEq)]
pub struct PostgresConfig {
    #[envconfig(env = "POSTGRES_USERNAME")]
    pub postgres_username: String,
    #[envconfig(env = "POSTGRES_PASSWORD")]
    pub postgres_password: String,
    #[envconfig(env = "POSTGRES_PORT")]
    pub postgres_port: u16,
    #[envconfig(env = "POSTGRES_NAME")]
    pub postgres_name: String,
    #[envconfig(env = "POSTGRES_HOST")]
    pub postgres_host: String,
    #[envconfig(env = "POSTGRES_SSL", default = "false")]
    pub postgres_ssl: bool,
    #[envconfig(env = "POSTGRES_WAIT_TIMEOUT_IN_MILLIS", default = "1000")]
    pub postgres_timeout: u64,
    #[envconfig(env = "POSTGRES_POOL_SIZE", default = "10")]
    pub postgres_pool_size: u32,
}

impl Display for PostgresConfig {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "POSTGRES_USER: ****")?;
        writeln!(f, "POSTGRES_PASSWORD: ****")?;
        writeln!(f, "POSTGRES_PORT: ****")?;
        writeln!(f, "POSTGRES_HOST: ****")?;
        writeln!(f, "POSTGRES_NAME: {}", self.postgres_name)?;
        writeln!(f, "POSTGRES_SSL: {}", self.postgres_ssl)?;
        writeln!(
            f,
            "POSTGRES_WAIT_TIMEOUT_IN_MILLIS: {}",
            self.postgres_timeout
        )?;
        writeln!(f, "POSTGRES_POOL_SIZE: {}", self.postgres_pool_size)
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
