use crate::{
    client::secrets_client::SecretsClient,
    prelude::{create_secret_response::CreateSecretResponse, get_secret_request::GetSecretRequest},
    ErrorMeta, IntegrationOSError, InternalError,
};
use async_trait::async_trait;
use serde_json::Value;

#[async_trait]
pub trait CryptoExt<R = CreateSecretResponse, A = GetSecretRequest> {
    async fn encrypt(&self, key: String, value: &Value) -> Result<R, IntegrationOSError>;
    async fn decrypt(&self, secret: &A) -> Result<Value, IntegrationOSError>;
}

#[async_trait]
impl CryptoExt for SecretsClient {
    async fn decrypt(
        &self,
        secret: &GetSecretRequest,
    ) -> Result<serde_json::Value, IntegrationOSError> {
        self.get_secret(secret).await.map_err(|e| {
            InternalError::encryption_error(e.message().as_ref(), Some("Failed to decrypt secret"))
        })
    }

    async fn encrypt(
        &self,
        key: String,
        value: &serde_json::Value,
    ) -> Result<CreateSecretResponse, IntegrationOSError> {
        self.create_secret(key, value).await.map_err(|e| {
            InternalError::encryption_error(e.message().as_ref(), Some("Failed to encrypt secret"))
        })
    }
}
