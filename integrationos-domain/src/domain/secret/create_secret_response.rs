use super::{SecretAuthor, SecretVersion};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CreateSecretResponse {
    #[serde(rename = "_id")]
    pub id: String,
    pub buildable_id: String,
    pub created_at: i64,
    pub author: SecretAuthor,
    pub encrypted_secret: String,
    pub version: SecretVersion,
}
