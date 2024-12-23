pub mod database_secret;
pub mod hashed_secret;
pub mod oauth_secret;

use crate::{IntegrationOSError, InternalError};
use chrono::Utc;
use secrecy::SecretString;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SecretAuthor {
    #[serde(rename = "_id")]
    pub id: String,
}

impl Default for SecretAuthor {
    fn default() -> Self {
        Self {
            id: "anonymous".to_string(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Default, Eq, PartialEq, Hash, Copy)]
#[serde(rename_all = "kebab-case")]
pub enum SecretVersion {
    #[default]
    // Refers to Google Cloud KMS
    V1,
    // Refers to Ios KMS
    V2,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Secret {
    #[serde(rename = "_id")]
    id: String,
    buildable_id: String,
    // Note: this was done due to an initial error making the timestamp as an f64
    created_at: f64,
    author: SecretAuthor,
    encrypted_secret: String,
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
            created_at: created_at.unwrap_or(Utc::now().timestamp_millis()) as f64,
            author: SecretAuthor::default(),
            encrypted_secret: secret,
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
        serde_json::from_str(&self.encrypted_secret)
            .map_err(|e| InternalError::deserialize_error(&e.to_string(), None))
    }

    pub fn as_value(&self) -> Result<Value, IntegrationOSError> {
        self.decode()
            .or_else(|_| serde_json::to_value(&self.encrypted_secret))
            .map_err(|e| InternalError::deserialize_error(&e.to_string(), None))
    }

    pub fn version(&self) -> Option<SecretVersion> {
        self.version
    }

    pub fn created_at(&self) -> i64 {
        self.created_at as i64
    }

    pub fn buildable_id(&self) -> String {
        self.buildable_id.clone()
    }

    pub fn encrypted_secret(&self) -> SecretString {
        SecretString::from(self.encrypted_secret.clone())
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use serde_json::json;

    #[test]
    fn test_should_deserialize_secret_as_value() {
        let secret = Secret::new(
            "brand_new_secret".to_string(),
            None,
            "buildable_id".to_string(),
            None,
        );
        let value = secret.as_value().unwrap();
        assert_eq!(value, json!("brand_new_secret"));
    }

    #[test]
    fn test_should_deserialize_secret_as_json() {
        let secret = json!({"SECRET_KEY": "brand_new_secret"});
        let secret = Secret::new(secret.to_string(), None, "buildable_id".to_string(), None);
        let value = secret.as_value().unwrap();
        assert_eq!(value, json!({"SECRET_KEY": "brand_new_secret"}));
    }

    #[test]
    fn test_should_deserialize_secret_as_a_custom_type() {
        #[derive(Deserialize, Serialize)]
        struct CustomSecret {
            secret_key: String,
        }
        let secret = CustomSecret {
            secret_key: "brand_new_secret".to_string(),
        };
        let secret = serde_json::to_value(secret).expect("Failed to serialize secret");

        let secret = Secret::new(secret.to_string(), None, "buildable_id".to_string(), None);
        let custom_secret: CustomSecret = secret.decode().expect("Failed to decode secret");
        assert_eq!(custom_secret.secret_key, "brand_new_secret");
    }
}
