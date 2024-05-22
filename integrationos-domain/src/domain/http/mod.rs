use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Hash)]
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
