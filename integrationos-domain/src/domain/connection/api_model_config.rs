use http::HeaderMap;
use js_sandbox_ios::Script;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

use crate::{prelude::schema::json_schema::JsonSchema, IntegrationOSError, InternalError};

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[cfg_attr(feature = "dummy", derive(fake::Dummy))]
#[serde(rename_all = "camelCase")]
pub struct ApiModelConfig {
    pub base_url: String,
    pub path: String,
    pub auth_method: AuthMethod,
    #[serde(
        with = "http_serde_ext_ios::header_map::option",
        skip_serializing_if = "Option::is_none",
        default
    )]
    pub headers: Option<HeaderMap>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query_params: Option<BTreeMap<String, String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content: Option<ContentType>,
    pub schemas: SchemasInput,
    pub samples: SamplesInput,
    pub responses: Vec<ResponseBody>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub paths: Option<ModelPaths>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, Default)]
#[cfg_attr(feature = "dummy", derive(fake::Dummy))]
#[serde(rename_all = "camelCase")]
/// This type is a combination of the `Method` and the `ContentType` of the request
pub enum ContentType {
    Json,
    Form,
    #[default]
    Other,
}

impl ApiModelConfig {
    /// Returns the full path of the API endpoint
    /// e.g. https://api.example.com/v1/users
    pub fn uri(&self) -> String {
        let mut base_url = self.base_url.to_owned();
        let path = self.path.to_owned();
        if base_url.ends_with('/') && path.starts_with('/') {
            base_url.pop();
        }
        base_url + &path
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[cfg_attr(feature = "dummy", derive(fake::Dummy))]
pub struct ModelPaths {
    pub request: Option<RequestModelPaths>,
    pub response: Option<ResponseModelPaths>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[cfg_attr(feature = "dummy", derive(fake::Dummy))]
#[serde(rename_all = "camelCase")]
pub struct RequestModelPaths {
    pub object: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[cfg_attr(feature = "dummy", derive(fake::Dummy))]
#[serde(rename_all = "camelCase")]
pub struct ResponseModelPaths {
    pub object: Option<String>,
    pub id: Option<String>,
    pub cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "dummy", derive(fake::Dummy))]
#[serde(rename_all = "camelCase")]
pub struct SamplesInput {
    #[serde(
        with = "http_serde_ext_ios::header_map::option",
        skip_serializing_if = "Option::is_none",
        default
    )]
    pub headers: Option<http::HeaderMap>,
    pub query_params: Option<Value>,
    pub path_params: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "dummy", derive(fake::Dummy))]
#[serde(rename_all = "camelCase")]
pub struct SchemasInput {
    pub headers: Option<JsonSchema>,
    pub query_params: Option<JsonSchema>,
    pub path_params: Option<JsonSchema>,
    pub body: Option<JsonSchema>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "dummy", derive(fake::Dummy))]
#[serde(rename_all = "camelCase")]
pub struct ResponseBody {
    pub status_code: u16,
    #[serde(
        with = "http_serde_ext_ios::header_map::option",
        skip_serializing_if = "Option::is_none",
        default
    )]
    pub headers: Option<http::HeaderMap>,
    pub body: Option<Value>,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Deserialize, Serialize)]
#[cfg_attr(feature = "dummy", derive(fake::Dummy))]
#[serde(tag = "type")]
pub enum AuthMethod {
    BearerToken {
        value: String,
    },
    ApiKey {
        key: String,
        value: String,
    },
    BasicAuth {
        username: String,
        password: String,
    },
    OAuthLegacy {
        #[serde(rename = "hashAlgorithm")]
        hash_algorithm: OAuthLegacyHashAlgorithm,
        #[serde(skip_serializing_if = "Option::is_none")]
        realm: Option<String>,
    },
    OAuth,
    None,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Deserialize, Serialize)]
#[cfg_attr(feature = "dummy", derive(fake::Dummy))]
pub enum OAuthLegacyHashAlgorithm {
    #[serde(rename = "HMAC-SHA1")]
    HmacSha1,
    #[serde(rename = "HMAC-SHA256")]
    HmacSha256,
    #[serde(rename = "HMAC-SHA512")]
    HmacSha512,
    #[serde(rename = "PLAINTEXT")]
    PlainText,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Function(pub Compute);

#[cfg(feature = "dummy")]
impl<T> fake::Dummy<T> for Function {
    fn dummy_with_rng<R: rand::prelude::Rng + ?Sized>(_: &T, _: &mut R) -> Self {
        use fake::{Fake, Faker};
        Function(Compute {
            entry: Faker.fake(),
            function: Faker.fake(),
            language: Lang::JavaScript,
        })
    }
}

impl Function {
    pub fn compute<T: DeserializeOwned>(&self, payload: &Value) -> Result<T, IntegrationOSError> {
        let mut js_code = Script::from_string(&self.0.function).map_err(|e| {
            InternalError::script_error(
                &e.to_string(),
                Some("Failed to create script from function"),
            )
        })?;
        let response: T = js_code.call(&self.0.entry, (payload,)).map_err(|e| {
            InternalError::script_error(&e.to_string(), Some("Failed to call function"))
        })?;

        Ok(response)
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Deserialize, Serialize)]
#[cfg_attr(feature = "dummy", derive(fake::Dummy))]
#[serde(rename_all = "camelCase")]
pub struct Compute {
    pub entry: String,
    pub function: String,
    pub language: Lang,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Deserialize, Serialize, Default)]
#[cfg_attr(feature = "dummy", derive(fake::Dummy))]
#[serde(rename_all = "lowercase")]
pub enum Lang {
    #[default]
    JavaScript,
    TypeScript,
    Rust,
}
