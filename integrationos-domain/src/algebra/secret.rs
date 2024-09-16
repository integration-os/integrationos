use super::{CryptoExt, GoogleCryptoKms, IOSCrypto, MongoStore};
use crate::{
    prelude::secret::Secret, secrets::SecretsConfig, IntegrationOSError, InternalError,
    SecretVersion,
};
use async_trait::async_trait;
use bson::doc;
use secrecy::ExposeSecret;
use serde_json::Value;

#[async_trait]
pub trait SecretExt {
    async fn get(&self, id: &str, buildable_id: &str) -> Result<Secret, IntegrationOSError>;

    async fn create(
        &self,
        secret: &Value,
        buildable_id: &str,
    ) -> Result<Secret, IntegrationOSError>;
}

#[derive(Debug, Clone)]
pub struct IOSKms {
    storage: MongoStore<Secret>,
    crypto: IOSCrypto,
}

impl IOSKms {
    pub async fn new(
        secrets_config: &SecretsConfig,
        storage: MongoStore<Secret>,
    ) -> Result<Self, IntegrationOSError> {
        let crypto = IOSCrypto::new(secrets_config.clone())?;
        Ok(Self { crypto, storage })
    }
}

#[async_trait]
impl SecretExt for IOSKms {
    async fn get(&self, id: &str, buildable_id: &str) -> Result<Secret, IntegrationOSError> {
        let secret = self
            .storage
            .get_one(doc! { "_id": id, "buildableId": buildable_id })
            .await?
            .ok_or_else(|| InternalError::key_not_found("Secret", None))?;

        let encrypted_secret = secret.encrypted_secret().expose_secret().to_owned();
        let version = secret.version();

        let decrypted_secret = self.crypto.decrypt(encrypted_secret, version).await?;

        Ok(Secret::new(
            decrypted_secret,
            secret.version(),
            secret.buildable_id(),
            Some(secret.created_at()),
        ))
    }

    async fn create(
        &self,
        secret: &Value,
        buildable_id: &str,
    ) -> Result<Secret, IntegrationOSError> {
        let string = serde_json::to_string(&secret).map_err(|_| {
            InternalError::serialize_error("The provided value is not a valid UTF-8 string", None)
        })?;

        let encrypted_secret = self.crypto.encrypt(string).await?;

        let secret = Secret::new(
            encrypted_secret,
            Some(SecretVersion::V2),
            buildable_id.to_owned(),
            None,
        );

        self.storage
            .create_one(&secret)
            .await
            .map_err(|e| InternalError::io_err(e.as_ref(), None))?;

        Ok(secret)
    }
}

#[derive(Debug, Clone)]
pub struct GoogleKms {
    storage: MongoStore<Secret>,
    crypto: GoogleCryptoKms,
}

impl GoogleKms {
    pub async fn new(
        secrets_config: &SecretsConfig,
        storage: MongoStore<Secret>,
    ) -> Result<Self, IntegrationOSError> {
        let crypto = GoogleCryptoKms::new(secrets_config).await?;
        Ok(Self { crypto, storage })
    }
}

#[async_trait]
impl SecretExt for GoogleKms {
    async fn get(&self, id: &str, buildable_id: &str) -> Result<Secret, IntegrationOSError> {
        let secret = self
            .storage
            .get_one(doc! { "_id": id, "buildableId": buildable_id })
            .await?
            .ok_or_else(|| InternalError::key_not_found("Secret", None))?;

        let encrypted_secret = secret.encrypted_secret().expose_secret().to_owned();

        let version = secret.version();

        let decrypted_secret = self.crypto.decrypt(encrypted_secret, version).await?;

        Ok(Secret::new(
            decrypted_secret,
            secret.version(),
            secret.buildable_id(),
            Some(secret.created_at()),
        ))
    }

    async fn create(
        &self,
        secret: &Value,
        buildable_id: &str,
    ) -> Result<Secret, IntegrationOSError> {
        let string = serde_json::to_string(&secret).map_err(|_| {
            InternalError::serialize_error("The provided value is not a valid UTF-8 string", None)
        })?;
        let encrypted_secret = self.crypto.encrypt(string).await?;

        let secret = Secret::new(
            encrypted_secret,
            Some(SecretVersion::V2),
            buildable_id.to_owned(),
            None,
        );

        self.storage
            .create_one(&secret)
            .await
            .map_err(|e| InternalError::io_err(e.as_ref(), None))?;

        Ok(secret)
    }
}
