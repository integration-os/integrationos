pub mod create_secret_request;
pub mod create_secret_response;
pub mod get_secret_request;
pub mod get_secret_response;
pub mod hashed_secret;
pub mod oauth_secret;

use serde::{Deserialize, Serialize};

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
    // Refers to inhouse encryption
    V2,
}
