use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Cursor {
    #[serde(rename = "_id")]
    pub id: String,
    pub key: String,
    pub value: String,
}
