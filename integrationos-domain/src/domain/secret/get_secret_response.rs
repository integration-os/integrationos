use super::{SecretAuthor, SecretVersion};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetSecretResponse<T> {
    #[serde(rename = "_id")]
    pub id: String,
    pub buildable_id: String,
    pub created_at: i64,
    pub author: SecretAuthor,
    pub secret: T,
    #[serde(default)]
    pub version: Option<SecretVersion>,
}
