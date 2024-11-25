use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::policies::Policies;

#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq)]
#[cfg_attr(feature = "dummy", derive(fake::Dummy))]
#[serde(rename_all = "camelCase")]
pub struct HttpExtractor {
    pub key: String,
    pub url: String,
    #[serde(with = "http_serde_ext_ios::method")]
    pub method: http::Method,
    pub headers: String,
    pub data: String,
    pub policies: Policies,
    pub start_to_close_timeout: String,
    #[serde(skip)]
    #[cfg_attr(feature = "dummy", dummy(default))]
    pub context: Option<Value>,
    #[serde(skip)]
    #[cfg_attr(feature = "dummy", dummy(default))]
    pub auth_token: Option<String>,
}
