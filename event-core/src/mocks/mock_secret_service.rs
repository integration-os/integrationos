use anyhow::Result;
use async_trait::async_trait;
use integrationos_domain::{
    algebra::crypto::Crypto, create_secret_response::CreateSecretResponse,
    get_secret_request::GetSecretRequest, IntegrationOSError,
};

#[derive(Debug, Clone)]
pub struct MockSecretsClient;

#[async_trait]
impl Crypto for MockSecretsClient {
    async fn decrypt(
        &self,
        _secret: &GetSecretRequest,
    ) -> Result<serde_json::Value, IntegrationOSError> {
        Ok(serde_json::Value::Null)
    }

    async fn encrypt(
        &self,
        _key: String,
        _val: &serde_json::Value,
    ) -> Result<CreateSecretResponse, IntegrationOSError> {
        unimplemented!()
    }
}
