use std::fmt::{Display, Formatter};

use envconfig::Envconfig;

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
