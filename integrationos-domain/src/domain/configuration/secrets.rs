use std::fmt::{Display, Formatter, Result};

use envconfig::Envconfig;

#[derive(Debug, Clone, Envconfig)]
pub struct SecretsConfig {
    #[envconfig(
        from = "SECRETS_SERVICE_BASE_URL",
        default = "https://secrets-service-development-b2nnzrt2eq-uk.a.run.app/"
    )]
    pub base_url: String,
    #[envconfig(from = "SECRETS_SERVICE_GET_PATH", default = "v1/secrets/get/")]
    pub get_path: String,
    #[envconfig(from = "SECRETS_SERVICE_CREATE_PATH", default = "v1/secrets/create/")]
    pub create_path: String,
}

impl SecretsConfig {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Default for SecretsConfig {
    fn default() -> Self {
        Self {
            base_url: "https://secrets-service-development-b2nnzrt2eq-uk.a.run.app/".to_owned(),
            get_path: "v1/secrets/get/".to_owned(),
            create_path: "v1/secrets/create/".to_owned(),
        }
    }
}

impl Display for SecretsConfig {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        writeln!(f, "SECRETS_SERVICE_BASE_URL: {}", self.base_url)?;
        writeln!(f, "SECRETS_SERVICE_GET_PATH: {}", self.get_path)?;
        writeln!(f, "SECRETS_SERVICE_CREATE_PATH: {}", self.create_path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_config() {
        let config = SecretsConfig::new();

        assert_eq!(
            config.base_url,
            "https://secrets-service-development-b2nnzrt2eq-uk.a.run.app/"
        );
        assert_eq!(config.get_path, "v1/secrets/get/");
        assert_eq!(config.create_path, "v1/secrets/create/");
    }

    #[tokio::test]
    async fn test_config_display() {
        let config = SecretsConfig::new();

        let config_str = format!("{config}");

        let display = "SECRETS_SERVICE_BASE_URL: https://secrets-service-development-b2nnzrt2eq-uk.a.run.app/\n\
            SECRETS_SERVICE_GET_PATH: v1/secrets/get/\n\
            SECRETS_SERVICE_CREATE_PATH: v1/secrets/create/\n\
        ";

        assert_eq!(config_str, display);
    }
}
