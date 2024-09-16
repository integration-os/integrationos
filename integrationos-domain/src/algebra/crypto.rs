use crate::{secrets::SecretsConfig, IntegrationOSError, InternalError, SecretVersion};
use async_trait::async_trait;
use base64::{prelude::BASE64_STANDARD, Engine};
use chacha20poly1305::aead::generic_array::typenum::Unsigned;
use chacha20poly1305::aead::generic_array::GenericArray;
use chacha20poly1305::aead::{Aead, AeadCore, KeyInit, OsRng};
use chacha20poly1305::ChaCha20Poly1305;
use google_cloud_kms::{
    client::{Client, ClientConfig},
    grpc::kms::v1::DecryptRequest,
};
use secrecy::ExposeSecret;

#[async_trait]
pub trait CryptoExt {
    async fn encrypt(&self, encrypted_secret: String) -> Result<String, IntegrationOSError>;

    async fn decrypt(
        &self,
        data: String,
        version: Option<SecretVersion>,
    ) -> Result<String, IntegrationOSError>;
}

type NonceSize = <ChaCha20Poly1305 as AeadCore>::NonceSize;

#[derive(Debug, Clone)]
pub struct IOSCrypto {
    key: Vec<u8>,
}

#[async_trait]
impl CryptoExt for IOSCrypto {
    async fn encrypt(&self, encrypted_secret: String) -> Result<String, IntegrationOSError> {
        self.encrypt(encrypted_secret).await
    }

    async fn decrypt(
        &self,
        data: String,
        _: Option<SecretVersion>,
    ) -> Result<String, IntegrationOSError> {
        self.decrypt(data).await
    }
}

impl IOSCrypto {
    pub fn new(config: SecretsConfig) -> Result<Self, IntegrationOSError> {
        let len = config.ios_crypto_secret.expose_secret().as_bytes().len();

        if len != 32 {
            return Err(InternalError::invalid_argument(
                "The provided value is not a valid UTF-8 string",
                None,
            ));
        }

        let key: [u8; 32] = config
            .ios_crypto_secret
            .expose_secret()
            .as_bytes()
            .iter()
            .take(32)
            .map(|b| b.to_owned())
            .collect::<Vec<_>>()
            .try_into()
            .map_err(|_| {
                InternalError::invalid_argument(
                    "The provided value is not a valid UTF-8 string",
                    None,
                )
            })?;

        Ok(Self { key: key.to_vec() })
    }

    async fn decrypt(&self, encrypted_secret: String) -> Result<String, IntegrationOSError> {
        let obsf = hex::decode(encrypted_secret).map_err(|_| {}).map_err(|_| {
            InternalError::deserialize_error("The provided value is not a valid UTF-8 string", None)
        })?;
        let cipher = ChaCha20Poly1305::new(GenericArray::from_slice(&self.key));
        let (nonce, ciphertext) = obsf.split_at(NonceSize::to_usize());
        let nonce = GenericArray::from_slice(nonce);
        let plaintext = cipher.decrypt(nonce, ciphertext).map_err(|_| {
            InternalError::deserialize_error("The provided value is not a valid UTF-8 string", None)
        })?;
        let plaintext = String::from_utf8(plaintext).map_err(|_| {
            InternalError::deserialize_error("The provided value is not a valid UTF-8 string", None)
        })?;

        Ok(plaintext)
    }

    async fn encrypt(&self, secret: String) -> Result<String, IntegrationOSError> {
        let cipher = ChaCha20Poly1305::new(GenericArray::from_slice(&self.key));
        let nonce = ChaCha20Poly1305::generate_nonce(&mut OsRng);
        let mut obsf = cipher.encrypt(&nonce, secret.as_bytes()).map_err(|_| {
            InternalError::serialize_error("The provided value is not a valid UTF-8 string", None)
        })?;
        obsf.splice(..0, nonce.iter().copied());

        Ok(hex::encode(obsf))
    }
}

#[derive(Debug, Clone)]
pub struct GoogleCryptoKms {
    client: Client,
    config: SecretsConfig,
    fallback: IOSCrypto,
}

#[async_trait]
impl CryptoExt for GoogleCryptoKms {
    async fn encrypt(&self, encrypted_secret: String) -> Result<String, IntegrationOSError> {
        self.encrypt(encrypted_secret).await
    }

    async fn decrypt(
        &self,
        data: String,
        version: Option<SecretVersion>,
    ) -> Result<String, IntegrationOSError> {
        self.decrypt(data, version).await
    }
}

impl GoogleCryptoKms {
    pub async fn new(secrets_config: &SecretsConfig) -> Result<Self, IntegrationOSError> {
        let fallback = IOSCrypto::new(secrets_config.clone())?;
        let config = ClientConfig::default().with_auth().await.map_err(|e| {
            InternalError::connection_error(&e.to_string(), Some("Failed to create client"))
        })?;
        let client = Client::new(config).await.map_err(|e| {
            InternalError::connection_error(&e.to_string(), Some("Failed to create client"))
        })?;

        Ok(Self {
            client,
            config: secrets_config.clone(),
            fallback,
        })
    }

    async fn decrypt(
        &self,
        encrypted_secret: String,
        version: Option<SecretVersion>,
    ) -> Result<String, IntegrationOSError> {
        match version {
            Some(SecretVersion::V2) => self.fallback.decrypt(encrypted_secret).await,
            Some(SecretVersion::V1) | None => {
                let request = DecryptRequest {
                    name: format!(
                        "projects/{project_id}/locations/{location_id}/keyRings/{key_ring_id}/cryptoKeys/{key_id}",
                        project_id = self.config.google_kms_project_id,
                        location_id = self.config.google_kms_location_id,
                        key_ring_id = self.config.google_kms_key_ring_id,
                        key_id = self.config.google_kms_key_id,
                    ),
                    ciphertext: BASE64_STANDARD.decode(encrypted_secret.as_bytes())
                        .map_err(|_| InternalError::deserialize_error("The provided value is not a valid UTF-8 string", None))?,
                    ..Default::default()
                };

                let decriptes_bytes = self.client.decrypt(request, None).await.map_err(|_| {
                    InternalError::connection_error(
                        "The provided value is not a valid UTF-8 string",
                        None,
                    )
                })?;

                let plaintext = String::from_utf8(decriptes_bytes.plaintext).map_err(|_| {
                    InternalError::deserialize_error(
                        "The provided value is not a valid UTF-8 string",
                        None,
                    )
                })?;

                Ok(plaintext)
            }
        }
    }

    async fn encrypt(&self, secret: String) -> Result<String, IntegrationOSError> {
        // This is semantically incorrect. But support for Google encryption will be removed in the future, hence the lack of support for V1 encryption.
        self.fallback.encrypt(secret).await
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[tokio::test]
    async fn should_encrypt_and_decrypt_data() {
        let config = SecretsConfig::default();
        let crypto = IOSCrypto::new(config).expect("Failed to create IOSCrypto client");

        let data = "lorem_ipsum-dolor_sit-amet";
        let encrypted = crypto
            .encrypt(data.to_owned())
            .await
            .expect("Failed to encrypt data");
        let decrypted = crypto
            .decrypt(encrypted.to_owned())
            .await
            .expect("Failed to decrypt data");

        assert_eq!(data, decrypted);
    }

    #[tokio::test]
    async fn should_fail_to_decrypt_if_the_key_is_different() {
        let config = SecretsConfig::default();
        let crypto = IOSCrypto::new(config).expect("Failed to create IOSCrypto client");

        let data = "lorem_ipsum-dolor_sit-amet";
        let encrypted = crypto
            .encrypt(data.to_owned())
            .await
            .expect("Failed to encrypt data");

        let config = SecretsConfig::new().with_secret("lorem_ipsum-dolor_sit_amet-neque".into());
        let crypto = IOSCrypto::new(config).expect("Failed to create IOSCrypto client");

        let decrypted = crypto.decrypt(encrypted).await;

        assert!(decrypted.is_err());
    }

    #[tokio::test]
    async fn should_fail_to_decrypt_if_the_data_is_tampered() {
        let config = SecretsConfig::default();
        let crypto = IOSCrypto::new(config).expect("Failed to create IOSCrypto client");

        let data = "lorem_ipsum-dolor_sit-amet";
        let encrypted = crypto
            .encrypt(data.to_owned())
            .await
            .expect("Failed to encrypt data");

        let mut obsf = hex::decode(encrypted).expect("Failed to decode encrypted data");
        obsf[0] = 0;
        let tampered = hex::encode(obsf);

        let decrypted = crypto.decrypt(tampered).await;

        assert!(decrypted.is_err());
    }
}
