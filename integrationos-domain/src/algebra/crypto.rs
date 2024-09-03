use crate::{
    create_secret_request::CreateSecretRequest,
    create_secret_response::CreateSecretAuthor,
    prelude::{create_secret_response::CreateSecretResponse, get_secret_request::GetSecretRequest},
    secrets::SecretsConfig,
    service::client::secrets_client::SecretsClient,
    ErrorMeta, IntegrationOSError, InternalError, Store,
};
use async_trait::async_trait;
use chrono::Utc;
use google_cloud_kms::{
    client::{Client, ClientConfig},
    grpc::kms::v1::EncryptRequest,
};
use serde::Serialize;
use serde_json::Value;
use uuid::Uuid;

use super::MongoStore;

#[async_trait]
pub trait CryptoExt<R = CreateSecretResponse, A = GetSecretRequest> {
    async fn encrypt(
        &self,
        key: String,
        value: &serde_json::Value,
    ) -> Result<R, IntegrationOSError>;
    async fn decrypt(&self, secret: &A) -> Result<serde_json::Value, IntegrationOSError>;
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

    async fn get_secret(
        &self,
        secret: &GetSecretRequest,
    ) -> Result<serde_json::Value, IntegrationOSError> {
        // let response = self.client.get_secret(secret).await.map_err(|e| {
        //     InternalError::connection_error(&e.to_string(), Some("Failed to get secret"))
        // })?;

        // Ok(response.payload.data)

        // const keyName = client.cryptoKeyPath(
        //         projectId,
        //         locationId,
        //         keyRingId,
        //         keyId,
        //       );

        // const [decryptResponse] = await client.decrypt({
        //         name: keyName,
        //         ciphertext: encryptResponse.ciphertext,
        //         ciphertextCrc32c: {
        //           value: encryptResponse.ciphertextCrc32c,
        //         },
        //       });

        // let decrypt_response = self.client.decrypt(key_name, encrypt_response.ciphertext).await.map_err(|e| {
        //     InternalError::connection_error(&e.to_string(), Some("Failed to decrypt secret"))
        // })?;

        // Ok(decrypt_response.plaintext)

        // self.client.create_crypto_key(req, retry)

        unimplemented!()
    }

    async fn create_secret<T: Serialize>(
        &self,
        secret: &CreateSecretRequest<T>, // TODO: Use CreateSecretRequest<T> instead of CreateSecretRequest
    ) -> Result<CreateSecretResponse, IntegrationOSError> {
        // let response = self.client.create_secret(secret).await.map_err(|e| {
        //     InternalError::connection_error(&e.to_string(), Some("Failed to create secret"))
        // })?;

        // Ok(response)

        // cryptoKeyPathTemplate: new this._gaxModule.PathTemplate('projects/{project}/locations/{location}/keyRings/{key_ring}/cryptoKeys/{crypto_key}'),
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

        // const [encryptResponse] = await client.encrypt({
        //         name: keyName,
        //         plaintext: plaintextBuffer,
        //         plaintextCrc32c: {
        //           value: plaintextCrc32c,
        //         },
        //       });
        let encrypt_response = self.client.encrypt(request, None).await.map_err(|e| {
            InternalError::connection_error(&e.to_string(), Some("Failed to encrypt secret"))
        })?;

        let ciphertext = encrypt_response.ciphertext;

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
                author: CreateSecretAuthor {
                    id: secret.buildable_id.clone(),
                },
                encrypted_secret: String::from_utf8(ciphertext)
                    .map_err(|e| InternalError::invalid_argument(&e.to_string(), None))?,
            };

            self.storage.create_one(&secret).await?;
            // Ok(ciphertext)
            todo!()
        }

        // if encrypt_response.ciphertext_crc32c != checksum as i64 {
        //     return Err(InternalError::invalid_argument("Invalid ciphertext crc32c", None));
        // }
    }
}

#[async_trait]
impl CryptoExt for GoogleCryptoKms {
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
