use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateSecretRequest<T> {
    pub buildable_id: String,
    pub secret: T,
}
