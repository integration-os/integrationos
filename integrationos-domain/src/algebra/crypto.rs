use crate::{
    create_secret_request::CreateSecretRequest,
    prelude::{create_secret_response::CreateSecretResponse, get_secret_request::GetSecretRequest},
    secrets::SecretsConfig,
    service::client::secrets_client::SecretsClient,
    ErrorMeta, IntegrationOSError, InternalError,
};
use async_trait::async_trait;
use google_cloud_kms::client::{Client, ClientConfig};
use serde::Serialize;

#[async_trait]
pub trait CryptoExt<R = CreateSecretResponse, A = GetSecretRequest> {
    async fn encrypt(
        &self,
        key: String,
        value: &serde_json::Value,
    ) -> Result<R, IntegrationOSError>;
    async fn decrypt(&self, secret: &A) -> Result<serde_json::Value, IntegrationOSError>;
}

pub struct GoogleSecretKms {
    client: Client,
    config: SecretsConfig,
}

impl GoogleSecretKms {
    pub async fn new(secrets_config: &SecretsConfig) -> Result<Self, IntegrationOSError> {
        let config = ClientConfig::default().with_auth().await.map_err(|e| {
            InternalError::connection_error(&e.to_string(), Some("Failed to create client"))
        })?;
        let client = Client::new(config).await.map_err(|e| {
            InternalError::connection_error(&e.to_string(), Some("Failed to create client"))
        })?;

        Ok(Self {
            client,
            config: secrets_config.clone(),
        })
    }

    async fn get_secret(
        &self,
        secret: &GetSecretRequest,
    ) -> Result<serde_json::Value, IntegrationOSError> {
        // let response = self.client.get_secret(secret).await.map_err(|e| {
        //     InternalError::connection_error(&e.to_string(), Some("Failed to get secret"))
        // })?;

        // Ok(response.payload.data)

        unimplemented!()
    }

    async fn create_secret<T: Serialize>(
        &self,
        secret: &CreateSecretRequest<T>,
    ) -> Result<CreateSecretResponse, IntegrationOSError> {
        // let response = self.client.create_secret(secret).await.map_err(|e| {
        //     InternalError::connection_error(&e.to_string(), Some("Failed to create secret"))
        // })?;

        // Ok(response)

        unimplemented!()
    }
}

#[async_trait]
impl CryptoExt for GoogleSecretKms {
    async fn encrypt(
        &self,
        key: String,
        value: &serde_json::Value,
    ) -> Result<CreateSecretResponse, IntegrationOSError> {
        unimplemented!()
    }

    async fn decrypt(
        &self,
        secret: &GetSecretRequest,
    ) -> Result<serde_json::Value, IntegrationOSError> {
        unimplemented!()
    }
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
