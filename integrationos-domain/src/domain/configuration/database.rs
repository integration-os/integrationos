use super::environment::Environment;
use crate::{ApplicationError, IntegrationOSError};
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

#[derive(Envconfig, Clone, Serialize, Deserialize, PartialEq)]
pub struct DatabaseConnectionConfig {
    #[envconfig(from = "WORKER_THREADS")]
    pub worker_threads: Option<usize>,
    #[envconfig(from = "INTERNAL_SERVER_ADDRESS", default = "0.0.0.0:5005")]
    pub address: SocketAddr,
    #[envconfig(from = "ENVIRONMENT", default = "development")]
    pub environment: Environment,
    #[envconfig(from = "EMIT_URL", default = "http://localhost:3001")]
    pub emit_url: String,
    #[envconfig(nested = true)]
    pub postgres_config: PostgresConfig,
    #[envconfig(from = "DATABASE_CONNECTION_TYPE", default = "postgres")]
    pub database_connection_type: DatabaseConnectionType,
}

impl DatabaseConnectionConfig {
    /// Merges the unknown fields from the environment
    /// into the current config
    ///
    /// # Arguments
    /// * `other` - The unknown fields from the environment
    ///
    /// # Returns
    /// * `Result<Self, IntegrationOSError>` - The updated config
    pub fn merge_unknown(
        mut self,
        other: HashMap<String, String>,
    ) -> Result<Self, IntegrationOSError> {
        if let Some(worker_threads) = other.get("WORKER_THREADS") {
            self.worker_threads = Some(worker_threads.parse::<usize>().map_err(|e| {
                ApplicationError::bad_request(&format!("Invalid worker threads: {}", e), None)
            })?);
        }

        if let Some(address) = other.get("INTERNAL_SERVER_ADDRESS") {
            self.address = address.parse::<SocketAddr>().map_err(|e| {
                ApplicationError::bad_request(&format!("Invalid address: {}", e), None)
            })?;
        }

        if let Some(environment) = other.get("ENVIRONMENT") {
            self.environment = environment.parse().map_err(|e| {
                ApplicationError::bad_request(&format!("Invalid environment: {}", e), None)
            })?;
        }

        if let Some(database_connection_type) = other.get("DATABASE_CONNECTION_TYPE") {
            self.database_connection_type = database_connection_type.parse().map_err(|e| {
                ApplicationError::bad_request(
                    &format!("Invalid database connection type: {}", e),
                    None,
                )
            })?;
        };

        // if connection type is postgres, get all the fields for postgres config
        match self.database_connection_type {
            DatabaseConnectionType::PostgreSql => {
                // get all the fields for postgres config
                let mut postgres_config: HashMap<String, String> = HashMap::new();
                for (key, value) in other {
                    if key.starts_with("POSTGRES_") {
                        postgres_config.insert(key, value.to_string());
                    }
                }

                self.postgres_config = PostgresConfig::init_from_hashmap(&postgres_config)
                    .map_err(|e| {
                        ApplicationError::bad_request(
                            &format!("Invalid postgres config: {}", e),
                            None,
                        )
                    })?;
            }
        }

        Ok(self)
    }

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
        map.insert("ENVIRONMENT".to_string(), self.environment.to_string());
        map.insert(
            "POSTGRES_USERNAME".to_string(),
            self.postgres_config.postgres_username.clone(),
        );
        map.insert(
            "POSTGRES_PASSWORD".to_string(),
            self.postgres_config.postgres_password.clone(),
        );
        map.insert(
            "POSTGRES_PORT".to_string(),
            self.postgres_config.postgres_port.to_string(),
        );
        map.insert(
            "POSTGRES_NAME".to_string(),
            self.postgres_config.postgres_name.clone(),
        );
        map.insert(
            "POSTGRES_HOST".to_string(),
            self.postgres_config.postgres_host.clone(),
        );
        map.insert(
            "POSTGRES_SSL".to_string(),
            self.postgres_config.postgres_ssl.to_string(),
        );
        map.insert(
            "POSTGRES_WAIT_TIMEOUT_IN_MILLIS".to_string(),
            self.postgres_config.postgres_timeout.to_string(),
        );
        map.insert(
            "POSTGRES_POOL_SIZE".to_string(),
            self.postgres_config.postgres_pool_size.to_string(),
        );
        map.insert(
            "DATABASE_CONNECTION_TYPE".to_string(),
            self.database_connection_type.as_ref().into(),
        );

        map
    }
}

impl Default for DatabaseConnectionConfig {
    fn default() -> Self {
        Self {
            worker_threads: Some(1),
            emit_url: "http://localhost:3001".to_string(),
            address: SocketAddr::new("0.0.0.0".parse().expect("Invalid address"), 5005),
            environment: Environment::Development,
            postgres_config: PostgresConfig::default(),
            database_connection_type: DatabaseConnectionType::PostgreSql,
        }
    }
}

impl Display for DatabaseConnectionConfig {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "WORKER_THREADS: {:?}", self.worker_threads)?;
        writeln!(f, "INTERNAL_SERVER_ADDRESS: {}", self.address)?;
        writeln!(f, "EMIT_URL: {}", self.emit_url)?;
        writeln!(f, "{}", self.environment)?;
        match self.database_connection_type {
            DatabaseConnectionType::PostgreSql => writeln!(f, "{}", self.postgres_config),
        }
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
