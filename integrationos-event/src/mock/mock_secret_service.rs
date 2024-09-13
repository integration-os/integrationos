use anyhow::Result;
use async_trait::async_trait;
use integrationos_domain::{algebra::CryptoExt, IntegrationOSError, SecretVersion};

#[derive(Debug, Clone)]
pub struct MockSecretsClient;

#[async_trait]
impl CryptoExt for MockSecretsClient {
    async fn encrypt(&self, encrypted_secret: String) -> Result<String, IntegrationOSError> {
        Ok(encrypted_secret)
    }

    async fn decrypt(
        &self,
        data: String,
        _version: Option<SecretVersion>,
    ) -> Result<String, IntegrationOSError> {
        Ok(data)
    }
}
