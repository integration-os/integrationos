use serde::{Deserialize, Serialize};

use super::SecretVersion;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateSecretRequest<T> {
    pub buildable_id: String,
    pub secret: T,
    pub version: SecretVersion,
}
