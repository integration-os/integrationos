use super::IntegrationOSError;
use crate::InternalError;
use chrono::Utc;
use jsonwebtoken::{EncodingKey, Header};
use serde::{Deserialize, Serialize};

pub const DEFAULT_AUDIENCE: &str = "pica-users";
pub const DEFAULT_ISSUER: &str = "pica";

pub const FALLBACK_AUDIENCE: &str = "pica-users";
pub const FALLBACK_ISSUER: &str = "pica";

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Hash, Default)]
#[serde(rename_all = "camelCase")]
pub struct Claims {
    #[serde(rename = "_id")]
    pub id: String,
    pub email: String,
    pub username: String,
    pub user_key: String,
    pub first_name: String,
    pub last_name: String,
    pub buildable_id: String,
    pub container_id: String,
    pub pointers: Vec<String>,
    pub is_buildable_core: bool,
    pub iat: i64,
    pub exp: i64,
    pub aud: String,
    pub iss: String,
}

impl Claims {
    pub fn from_secret(secret: &str) -> Result<String, IntegrationOSError> {
        let now = Utc::now();

        let header = Header::default();
        let claims = Claims {
            is_buildable_core: true,
            iat: now.timestamp(),
            exp: now.timestamp() + 60,
            aud: DEFAULT_AUDIENCE.to_string(),
            iss: DEFAULT_ISSUER.to_string(),
            ..Default::default()
        };
        let key = EncodingKey::from_secret(secret.as_bytes());

        jsonwebtoken::encode(&header, &claims, &key).map_err(|e| {
            tracing::error!("Failed to encode token: {e}");
            InternalError::invalid_argument("Failed to encode token", None)
        })
    }
}
