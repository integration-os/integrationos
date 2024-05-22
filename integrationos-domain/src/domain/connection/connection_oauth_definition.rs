use super::api_model_config::{ApiModelConfig, Function};
use crate::{
    id::Id,
    prelude::{ownership::Ownership, shared::record_metadata::RecordMetadata},
    Feature, Hook,
};
use bson::doc;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::ops::Not;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "dummy", derive(fake::Dummy))]
#[serde(rename_all = "camelCase")]
pub struct ConnectionOAuthDefinition {
    #[serde(rename = "_id")]
    pub id: Id,
    pub configuration: OAuthApiConfig,
    pub connection_platform: String,
    pub compute: OAuthCompute,
    pub frontend: Frontend,
    #[serde(default, skip_serializing_if = "<&bool>::not")]
    pub is_full_template_enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub hooks: Option<Hook>,
    #[serde(flatten, default)]
    pub record_metadata: RecordMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "dummy", derive(fake::Dummy))]
#[serde(rename_all = "camelCase")]
pub struct OAuthCompute {
    pub init: ComputeRequest,
    pub refresh: ComputeRequest,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "dummy", derive(fake::Dummy))]
#[serde(rename_all = "camelCase")]
pub struct Computation {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query_params: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "dummy", derive(fake::Dummy))]
#[serde(rename_all = "camelCase")]
pub struct ComputeRequest {
    /// This function is guaranteed to return a Computation object.
    pub computation: Option<Function>,
    /// The blueprint to construct a OAuthResponse from the response of the oauth definition.
    pub response: Function,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "dummy", derive(fake::Dummy))]
#[serde(rename_all = "camelCase")]
pub struct Frontend {
    pub platform_redirect_uri: String,
    pub scopes: String,
    pub ios_redirect_uri: String,
    #[serde(skip_serializing_if = "Option::is_none", default = "default_separator")]
    pub separator: Option<String>,
}

fn default_separator() -> Option<String> {
    Some(String::from(" "))
}

#[derive(Deserialize, Debug, Clone, Serialize)]
#[cfg_attr(feature = "dummy", derive(fake::Dummy))]
#[serde(rename_all = "camelCase")]
pub struct OAuthResponse {
    pub access_token: String,
    pub expires_in: i32,
    pub refresh_token: Option<String>,
    pub token_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "dummy", derive(fake::Dummy))]
#[serde(rename_all = "camelCase")]
pub struct OAuthApiConfig {
    pub init: ApiModelConfig,
    pub refresh: ApiModelConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    #[serde(rename = "_id")]
    pub id: Id,
    pub ownership: Ownership,
    #[serde(default)]
    pub connected_platforms: Vec<ConnectedPlatform>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub features: Vec<Feature>,
}

impl Settings {
    pub fn platform_secret(&self, connection_definition_id: &Id) -> Option<String> {
        self.connected_platforms
            .iter()
            .find(|p| p.connection_definition_id == *connection_definition_id)
            .and_then(|p| p.secrets_service_id.clone())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectedPlatform {
    #[serde(rename = "type")]
    pub r#type: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scopes: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    pub connection_definition_id: Id,
    #[serde(default)]
    pub active: Option<bool>,
    pub image: Option<String>,
    pub secrets_service_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub secret: Option<ConnectedPlatformSecret>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectedPlatformSecret {
    client_id: String,
    client_secret_display: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub struct PlatformSecret {
    pub client_id: String,
    pub client_secret: String,
}
