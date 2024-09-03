use super::MongoStore;
use crate::{
    create_secret_request::CreateSecretRequest,
    get_secret_response::GetSecretResponse,
    prelude::{create_secret_response::CreateSecretResponse, get_secret_request::GetSecretRequest},
    secrets::SecretsConfig,
    IntegrationOSError, InternalError, SecretAuthor,
};
use base64::{prelude::BASE64_STANDARD, Engine};
use bson::doc;
use chrono::Utc;
use google_cloud_kms::{
    client::{Client, ClientConfig},
    grpc::kms::v1::{DecryptRequest, EncryptRequest},
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::future::Future;
use uuid::Uuid;

pub trait SecretExt {
    fn get<T: for<'a> Deserialize<'a>>(
        &self,
        secret: &GetSecretRequest,
    ) -> impl Future<Output = Result<GetSecretResponse<T>, IntegrationOSError>> + Send;
    fn create<T: Serialize>(
        &self,
        secret: &CreateSecretRequest<T>,
    ) -> impl Future<Output = Result<CreateSecretResponse, IntegrationOSError>>;
}

impl SecretExt for GoogleCryptoKms {
    async fn get<T: for<'a> Deserialize<'a>>(
        &self,
        secret: &GetSecretRequest,
    ) -> Result<GetSecretResponse<T>, IntegrationOSError> {
        self.get_secret(secret).await
    }

    async fn create<T: Serialize>(
        &self,
        secret: &CreateSecretRequest<T>,
    ) -> Result<CreateSecretResponse, IntegrationOSError> {
        self.create_secret(secret).await
    }
}

pub struct GoogleCryptoKms {
    client: Client,
    config: SecretsConfig,
    storage: MongoStore<Value>,
}

impl GoogleCryptoKms {
    pub async fn new(
        secrets_config: &SecretsConfig,
        storage: MongoStore<Value>,
    ) -> Result<Self, IntegrationOSError> {
        let config = ClientConfig::default().with_auth().await.map_err(|e| {
            InternalError::connection_error(&e.to_string(), Some("Failed to create client"))
        })?;
        let client = Client::new(config).await.map_err(|e| {
            InternalError::connection_error(&e.to_string(), Some("Failed to create client"))
        })?;

        Ok(Self {
            client,
            config: secrets_config.clone(),
            storage,
        })
    }

    async fn get_secret<T: for<'a> Deserialize<'a>>(
        &self,
        secret: &GetSecretRequest,
    ) -> Result<GetSecretResponse<T>, IntegrationOSError> {
        let encrypted_secret = self
            .storage
            .get_one(doc! { "_id": secret.id.clone(), "buildableId": secret.buildable_id.clone() })
            .await?
            .ok_or_else(|| InternalError::key_not_found("Secret", None))?;

        let encrypted_secret = serde_json::from_value::<CreateSecretResponse>(encrypted_secret)
            .map_err(|e| InternalError::deserialize_error(&e.to_string(), None))?;

        let request = DecryptRequest {
            name: format!(
                "projects/{project_id}/locations/{location_id}/keyRings/{key_ring_id}/cryptoKeys/{key_id}",
                project_id = self.config.google_kms_project_id,
                location_id = self.config.google_kms_location_id,
                key_ring_id = self.config.google_kms_key_ring_id,
                key_id = self.config.google_kms_key_id,
            ),
            ciphertext: BASE64_STANDARD.decode(encrypted_secret.encrypted_secret.as_bytes())
                .map_err(|e| InternalError::deserialize_error(&e.to_string(), None))?,
            ..Default::default()
        };

        let decrypted_secret = self.client.decrypt(request, None).await.map_err(|e| {
            InternalError::connection_error(&e.to_string(), Some("Failed to decrypt secret"))
        })?;

        let secret = GetSecretResponse {
            id: encrypted_secret.id.clone(),
            buildable_id: encrypted_secret.buildable_id.clone(),
            created_at: encrypted_secret.created_at,
            author: SecretAuthor::default(),
            secret: serde_json::from_slice::<T>(&decrypted_secret.plaintext)
                .map_err(|e| InternalError::deserialize_error(&e.to_string(), None))?,
            version: Some(encrypted_secret.version),
        };

        Ok(secret)
    }

    async fn create_secret<T: Serialize>(
        &self,
        secret: &CreateSecretRequest<T>,
    ) -> Result<CreateSecretResponse, IntegrationOSError> {
        // Follows node sdk implementation of cryptoKeyPathTemplate: new this._gaxModule.PathTemplate('projects/{project}/locations/{location}/keyRings/{key_ring}/cryptoKeys/{crypto_key}'),
        let key_name = format!(
            "projects/{project_id}/locations/{location_id}/keyRings/{key_ring_id}/cryptoKeys/{key_id}",
            project_id = self.config.google_kms_project_id,
            location_id = self.config.google_kms_location_id,
            key_ring_id = self.config.google_kms_key_ring_id,
            key_id = self.config.google_kms_key_id,
        );

        let checksum = crc32fast::hash(key_name.as_bytes());
        let request = EncryptRequest {
            name: key_name,
            plaintext: serde_json::to_vec(&secret.secret)
                .map_err(|e| InternalError::serialize_error(&e.to_string(), None))?,
            plaintext_crc32c: Some(checksum as i64),
            ..Default::default()
        };

        let encrypt_response = self.client.encrypt(request, None).await.map_err(|e| {
            InternalError::connection_error(&e.to_string(), Some("Failed to encrypt secret"))
        })?;

        let ciphertext = encrypt_response.ciphertext;
        let encoded = BASE64_STANDARD.encode(&ciphertext);

        if !encrypt_response.verified_plaintext_crc32c
            || crc32fast::hash(&ciphertext)
                != encrypt_response.ciphertext_crc32c.unwrap_or(0) as u32
        {
            Err(InternalError::invalid_argument(
                "Request corrupted in transport",
                None,
            ))
        } else {
            let secret = CreateSecretResponse {
                id: Uuid::new_v4().to_string(),
                buildable_id: secret.buildable_id.clone(),
                created_at: Utc::now().timestamp_millis(),
                author: SecretAuthor::default(),
                encrypted_secret: encoded,
                version: secret.version,
            };

            let value = serde_json::to_value(&secret)
                .map_err(|e| InternalError::serialize_error(&e.to_string(), None))?;

            self.storage.create_one(&value).await?;

            Ok(secret)
        }
    }
}
