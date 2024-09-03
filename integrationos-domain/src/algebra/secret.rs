use super::MongoStore;
use crate::{
    prelude::create_secret_response::Secret, secrets::SecretsConfig, IntegrationOSError,
    InternalError, SecretVersion,
};
use axum::async_trait;
use base64::{prelude::BASE64_STANDARD, Engine};
use bson::doc;
use google_cloud_kms::{
    client::{Client, ClientConfig},
    grpc::kms::v1::{DecryptRequest, EncryptRequest},
};
use secrecy::{ExposeSecret, SecretString};

#[async_trait]
pub trait SecretExt {
    async fn get(&self, id: String, buildable_id: String) -> Result<Secret, IntegrationOSError>;

    async fn create(
        &self,
        secret: &[u8],
        buildable_id: String,
        version: SecretVersion,
    ) -> Result<Secret, IntegrationOSError>;
}

#[async_trait]
impl SecretExt for GoogleCryptoKms {
    async fn get(&self, id: String, buildable_id: String) -> Result<Secret, IntegrationOSError> {
        self.get_secret(id, buildable_id).await
    }

    async fn create(
        &self,
        secret: &[u8],
        buildable_id: String,
        version: SecretVersion,
    ) -> Result<Secret, IntegrationOSError> {
        self.create_secret(secret, buildable_id, version).await
    }
}

pub struct GoogleCryptoKms {
    client: Client,
    config: SecretsConfig,
    storage: MongoStore<Secret>,
}

impl GoogleCryptoKms {
    pub async fn new(
        secrets_config: &SecretsConfig,
        storage: MongoStore<Secret>,
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
        // secret: &GetSecretRequest,
        id: String,
        buildable_id: String,
    ) -> Result<Secret, IntegrationOSError> {
        let encrypted_secret = self
            .storage
            .get_one(doc! { "_id": id.to_string().clone(), "buildableId": buildable_id.clone() })
            .await?
            .ok_or_else(|| InternalError::key_not_found("Secret", None))?;

        let request = DecryptRequest {
            name: format!(
                "projects/{project_id}/locations/{location_id}/keyRings/{key_ring_id}/cryptoKeys/{key_id}",
                project_id = self.config.google_kms_project_id,
                location_id = self.config.google_kms_location_id,
                key_ring_id = self.config.google_kms_key_ring_id,
                key_id = self.config.google_kms_key_id,
            ),
            ciphertext: BASE64_STANDARD.decode(encrypted_secret.secret().expose_secret().as_bytes())
                .map_err(|e| InternalError::deserialize_error(&e.to_string(), None))?,
            ..Default::default()
        };

        let decrypted_secret = self.client.decrypt(request, None).await.map_err(|e| {
            InternalError::connection_error(&e.to_string(), Some("Failed to decrypt secret"))
        })?;

        encrypted_secret.decrypted(SecretString::new(
            String::from_utf8(decrypted_secret.plaintext)
                .map_err(|e| InternalError::deserialize_error(&e.to_string(), None))?,
        ))
    }

    async fn create_secret(
        &self,
        secret: &[u8],
        buildable_id: String,
        version: SecretVersion,
    ) -> Result<Secret, IntegrationOSError> {
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
            plaintext: serde_json::to_vec(&secret)
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
            let secret = Secret::new(encoded, Some(version), buildable_id.clone(), None);

            self.storage.create_one(&secret).await?;

            Ok(secret)
        }
    }
}
