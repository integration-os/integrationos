use super::{SecretAuthor, SecretVersion};
use crate::{IntegrationOSError, InternalError};
use chrono::Utc;
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Secret {
    #[serde(rename = "_id")]
    id: String,
    buildable_id: String,
    created_at: i64,
    author: SecretAuthor,
    secret: String,
    #[serde(default)]
    version: Option<SecretVersion>,
}

impl Secret {
    pub fn new(
        secret: String,
        version: Option<SecretVersion>,
        buildable_id: String,
        created_at: Option<i64>,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            buildable_id,
            created_at: created_at.unwrap_or(Utc::now().timestamp_millis()),
            author: SecretAuthor::default(),
            secret,
            version,
        }
    }

    pub fn id(&self) -> String {
        self.id.clone()
    }

    pub fn decode<T>(&self) -> Result<T, IntegrationOSError>
    where
        T: for<'a> Deserialize<'a>,
    {
        serde_json::from_str(&self.secret)
            .map_err(|e| InternalError::deserialize_error(&e.to_string(), None))
    }

    pub fn version(&self) -> Option<SecretVersion> {
        self.version
    }

    pub fn created_at(&self) -> i64 {
        self.created_at
    }

    pub fn secret(&self) -> SecretString {
        SecretString::new(self.secret.clone())
    }

    pub fn decrypted(mut self, secret: SecretString) -> Result<Self, IntegrationOSError> {
        self.secret = secret.expose_secret().into();
        Ok(self)
    }
}
