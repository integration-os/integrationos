use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::prelude::connection_oauth_definition::OAuthResponse;

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct OAuthSecret {
    #[serde(rename = "OAUTH_CLIENT_ID")]
    pub client_id: String,
    #[serde(rename = "OAUTH_CLIENT_SECRET")]
    pub client_secret: String,
    #[serde(rename = "OAUTH_ACCESS_TOKEN")]
    pub access_token: String,
    #[serde(rename = "OAUTH_TOKEN_TYPE")]
    pub token_type: Option<String>,
    #[serde(rename = "OAUTH_REFRESH_TOKEN")]
    pub refresh_token: Option<String>,
    #[serde(rename = "OAUTH_EXPIRES_IN")]
    pub expires_in: i32,
    #[serde(rename = "OAUTH_METADATA")]
    pub metadata: Value,
    #[serde(rename = "OAUTH_REQUEST_PAYLOAD", default)]
    pub request_payload: Option<Value>,
}

impl OAuthSecret {
    pub fn from_init(
        oauth_response: OAuthResponse,
        client_id: String,
        client_secret: String,
        metadata: Value,
        request_payload: Option<Value>,
    ) -> Self {
        OAuthSecret {
            client_id,
            client_secret,
            access_token: oauth_response.access_token,
            token_type: oauth_response.token_type,
            refresh_token: oauth_response.refresh_token,
            expires_in: oauth_response.expires_in,
            metadata,
            request_payload,
        }
    }

    pub fn from_refresh(
        &self,
        oauth_response: OAuthResponse,
        client_id: Option<String>,
        client_secret: Option<String>,
        metadata: Value,
    ) -> Self {
        OAuthSecret {
            client_id: client_id.unwrap_or(self.client_id.clone()),
            client_secret: client_secret.unwrap_or(self.client_secret.clone()),
            access_token: oauth_response.access_token,
            token_type: oauth_response.token_type,
            refresh_token: oauth_response.refresh_token,
            expires_in: oauth_response.expires_in,
            metadata,
            request_payload: self.request_payload.clone(),
        }
    }

    pub fn as_json(&self) -> Value {
        serde_json::to_value(self).unwrap()
    }
}
