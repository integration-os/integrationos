use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateSecretResponse {
    #[serde(rename = "_id")]
    pub id: String,
    pub buildable_id: String,
    pub created_at: i64,
    pub author: CreateSecretAuthor,
    pub encrypted_secret: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateSecretAuthor {
    #[serde(rename = "_id")]
    pub id: String,
}
