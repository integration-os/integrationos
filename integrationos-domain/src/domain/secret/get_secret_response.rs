use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetSecretResponse<T> {
    #[serde(rename = "_id")]
    pub id: String,
    pub buildable_id: String,
    pub created_at: f64,
    pub author: GetSecretAuthor,
    pub secret: T,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GetSecretAuthor {
    #[serde(rename = "_id")]
    pub id: String,
}
