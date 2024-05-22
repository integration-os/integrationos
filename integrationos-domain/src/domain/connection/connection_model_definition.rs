use super::api_model_config::ApiModelConfig;
use crate::{
    id::Id,
    prelude::{schema::common_model::CommonModel, shared::record_metadata::RecordMetadata},
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use strum::{Display, EnumIter};

#[derive(Debug, Clone, Eq, PartialEq, Hash, Deserialize, Serialize)]
#[cfg_attr(feature = "dummy", derive(fake::Dummy))]
pub enum ParameterLocation {
    QueryParameter,
    RequestBody,
    Header,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[cfg_attr(feature = "dummy", derive(fake::Dummy))]
#[serde(rename_all = "camelCase")]
pub struct ConnectionModelDefinition {
    #[serde(rename = "_id")]
    pub id: Id,
    pub connection_platform: String,
    pub connection_definition_id: Id,
    pub platform_version: String,
    #[serde(default)]
    pub key: String,
    pub title: String,
    pub name: String,
    pub model_name: String,
    #[serde(with = "http_serde_ext::method")]
    #[cfg_attr(feature = "dummy", dummy(expr = "http::Method::GET"))]
    pub action: http::Method,
    pub action_name: CrudAction,

    #[serde(flatten)]
    pub platform_info: PlatformInfo,

    #[serde(flatten, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "dummy", dummy(default))]
    pub extractor_config: Option<ExtractorConfig>,

    pub test_connection_status: TestConnection,

    pub is_default_crud_mapping: Option<bool>,
    pub mapping: Option<CrudMapping>,

    #[serde(flatten, default)]
    pub record_metadata: RecordMetadata,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Deserialize, Serialize)]
#[cfg_attr(feature = "dummy", derive(fake::Dummy))]
#[serde(rename_all = "camelCase")]
pub struct TestConnection {
    pub last_tested_at: i64,
    pub state: TestConnectionState,
}

impl Default for TestConnection {
    fn default() -> Self {
        Self {
            last_tested_at: 0,
            state: TestConnectionState::Untested,
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Deserialize, Serialize, Default)]
#[cfg_attr(feature = "dummy", derive(fake::Dummy))]
#[serde(rename_all = "camelCase")]
pub enum TestConnectionState {
    Success {
        #[serde(rename = "requestPayload")]
        request_payload: String,
        response: String,
    },
    Failure {
        message: String,
        #[serde(rename = "requestPayload")]
        request_payload: String,
    },
    #[default]
    Untested,
}

pub enum ConnectionModelDefinitionWithState {
    Populated(ConnectionModelDefinition),
    Unpopulated(ConnectionModelDefinition),
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[cfg_attr(feature = "dummy", derive(fake::Dummy))]
#[serde(untagged)]
pub enum PlatformInfo {
    Api(ApiModelConfig),
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Deserialize, Serialize)]
#[cfg_attr(feature = "dummy", derive(fake::Dummy))]
#[serde(rename_all = "camelCase")]
pub struct ExtractorConfig {
    pub pull_frequency: i64,
    pub batch_size: i64,
    pub cursor: CursorConfig,
    pub limit: Option<LimitConfig>,
    pub sleep_after_finish: i64,
    pub update_config: Option<UpdateConfig>,
    pub enabled: bool,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Deserialize, Serialize)]
#[cfg_attr(feature = "dummy", derive(fake::Dummy))]
#[serde(rename_all = "camelCase")]
pub struct CursorConfig {
    pub param_name: Option<String>,
    pub location: Option<ParameterLocation>,
    pub format: Option<String>,
    pub cursor_path: String,
    pub data_path: String,
    pub js_extractor_function: Option<String>,
    pub reset_on_end: bool,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Deserialize, Serialize)]
#[cfg_attr(feature = "dummy", derive(fake::Dummy))]
#[serde(rename_all = "camelCase")]
pub struct LimitConfig {
    pub param_name: String,
    pub location: ParameterLocation,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Deserialize, Serialize)]
#[cfg_attr(feature = "dummy", derive(fake::Dummy))]
#[serde(rename_all = "camelCase")]
pub struct UpdateConfig {
    pub param_name: String,
    pub location: ParameterLocation,
    pub format: String,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Deserialize, Serialize)]
#[cfg_attr(feature = "dummy", derive(fake::Dummy))]
#[serde(rename_all = "camelCase")]
pub struct CrudMapping {
    pub action: CrudAction,
    pub common_model_name: String,
    pub from_common_model: Option<String>,
    pub to_common_model: Option<String>,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Deserialize, Serialize, Display, EnumIter)]
#[cfg_attr(feature = "dummy", derive(fake::Dummy))]
#[serde(rename_all = "camelCase")]
#[strum(serialize_all = "camelCase")]
pub enum CrudAction {
    GetOne,
    GetMany,
    GetCount,
    Update,
    Create,
    Delete,
    Custom,
}

impl CrudAction {
    pub fn description(&self) -> &'static str {
        match self {
            CrudAction::GetOne => "Get one record",
            CrudAction::GetMany => "List records",
            CrudAction::GetCount => "Get count of records",
            CrudAction::Update => "Update a record",
            CrudAction::Create => "Create a record",
            CrudAction::Delete => "Delete a record",
            CrudAction::Custom => "Custom action",
        }
    }

    pub fn example(&self, common_model: &CommonModel) -> Value {
        let meta = json!({
            "timestamp": chrono::offset::Local::now().timestamp_millis(),
            "latency": 36,
            "platformRateLimitRemaining": 0,
            "rateLimitRemaining": 0,
            "cache": {
              "hit": false,
              "ttl": 0,
              "key": ""
            },
            "transactionKey": "tx_ky::7fb4a1ee61454e61adf79f37251affc4",
            "txn": "727cc0ecc6144f6782ca0d72486d3dea",
            "platform": "PlatformName",
            "platformVersion": "v1",
            "connectionDefinitionKey": "conn_def::7923c71ff5ae4119870bc42182c50cb7",
            "action": self.to_string(),
            "commonModel": common_model.name,
            "commonModelVersion": common_model.record_metadata.version,
            "connectionKey": "platform::8e09f436-601a-40c4-97cd-f9347e924418",
            "hash": "3eafdfa35a39411ca4992e7139c69d854f2135ad956c45449a7b6a98288d59a61177f386801d460ca3ee3884635d575b",
            "heartbeats": [],
            "totalTransactions": 1
        });

        match self {
            CrudAction::Create => {
                json!({
                    "status": "success",
                    "statusCode": 200,
                    "unified": common_model.sample,
                    "passthrough": {},
                    "meta": meta
                })
            }
            CrudAction::GetMany => {
                json!({
                    "status": "success",
                    "statusCode": 200,
                    "unified": vec![common_model.clone().sample],
                    "passthrough": {},
                    "pagination": {
                        "cursor": "23e6534fa96e810b3",
                        "limit": 100
                    },
                    "meta": meta
                })
            }
            CrudAction::GetOne => {
                json!({
                    "status": "success",
                    "statusCode": 200,
                    "unified": common_model.sample,
                    "passthrough": {},
                    "meta": meta
                })
            }
            CrudAction::GetCount => {
                json!({
                    "status": "success",
                    "statusCode": 200,
                    "unified": {
                        "count": 1
                    },
                    "passthrough": {},
                    "meta": meta
                })
            }
            CrudAction::Update => {
                json!({
                    "status": "success",
                    "statusCode": 200,
                    "unified": {},
                    "passthrough": {},
                    "meta": meta
                })
            }
            CrudAction::Delete => {
                json!({
                    "status": "success",
                    "statusCode": 200,
                    "unified": {},
                    "passthrough": {},
                    "meta": meta
                })
            }
            CrudAction::Custom => {
                unimplemented!()
            }
        }
    }
}

#[cfg(test)]
mod tests {

    use crate::prelude::connection::api_model_config::AuthMethod;

    use super::*;
    use serde_json::json;

    #[test]
    fn test_deserialize_auth_method() {
        let bearer_token = json!({
            "type": "BearerToken",
            "value": "some_token"
        });

        let api_key = json!({
            "type": "ApiKey",
            "key": "X-Api-Key",
            "value": "some_key"
        });

        let basic_auth = json!({
            "type": "BasicAuth",
            "username": "username",
            "password": "password"
        });

        let deserialized_bearer_token: AuthMethod = serde_json::from_value(bearer_token).unwrap();
        let deserialized_api_key: AuthMethod = serde_json::from_value(api_key).unwrap();
        let deserialized_basic_auth: AuthMethod = serde_json::from_value(basic_auth).unwrap();

        assert_eq!(
            deserialized_bearer_token,
            AuthMethod::BearerToken {
                value: "some_token".to_string()
            }
        );
        assert_eq!(
            deserialized_api_key,
            AuthMethod::ApiKey {
                key: "X-Api-Key".to_string(),
                value: "some_key".to_string()
            }
        );
        assert_eq!(
            deserialized_basic_auth,
            AuthMethod::BasicAuth {
                username: "username".to_string(),
                password: "password".to_string()
            }
        );
    }

    #[test]
    fn test_deserialize_parameter_location() {
        let query_parameter = json!("QueryParameter");
        let request_body = json!("RequestBody");
        let header = json!("Header");

        let deserialized_query_parameter: ParameterLocation =
            serde_json::from_value(query_parameter).unwrap();
        let deserialized_request_body: ParameterLocation =
            serde_json::from_value(request_body).unwrap();
        let deserialized_header: ParameterLocation = serde_json::from_value(header).unwrap();

        assert_eq!(
            deserialized_query_parameter,
            ParameterLocation::QueryParameter
        );
        assert_eq!(deserialized_request_body, ParameterLocation::RequestBody);
        assert_eq!(deserialized_header, ParameterLocation::Header);
    }

    #[test]
    fn test_model_config_deserializing() {
        let sample_config = json!({
            "_id" : "conn_mod_def::AAAAAAAAAAA::AAAAAAAAAAAAAAAAAAAAAA",
            "connectionPlatform" : "stripe",
            "connectionDefinitionId" : "conn_def::AAAAAAAAAAA::AAAAAAAAAAAAAAAAAAAAAA",
            "platformVersion" : "v1",
            "title" : "Get Webhook Endpoints",
            "name" : "webhook_endpoints",
            "key" : "api::stripe::v1::Webhook::getOne::webhook_endpoints",
            "modelName" : "Webhook",
            "action" : "GET",
            "actionName": "getOne",
            "baseUrl" : "https://api.stripe.com/v1",
            "path" : "webhook_endpoints",
            "authMethod" : {
                "type" : "BearerToken",
                "value" : "stripe_secret_key"
            },
            "samples" : {
                "queryParams": null,
                "pathParams": null,
                "body": null
            },
            "schemas" : {
                "headers": null,
                "queryParams": null,
                "pathParams": null,
                "body": null
            },
            "paths": null,
            "responses": [],
            "headers" : null,
            "queryParams" : null,
            "pullFrequency" : 5,
            "batchSize" : 100,
            "cursor" : {
                "paramName" : "starting_after",
                "location" : "QueryParameter",
                "format" : "{cursor}",
                "cursorPath" : "_.body.id",
                "dataPath" : "_.body.data",
                "jsExtractorFunction" : null,
                "resetOnEnd" : true
            },
            "limit" : {
                "paramName" : "limit",
                "location" : "QueryParameter"
            },
            "sleepAfterFinish" : 86400,
            "updateConfig" : null,
            "enabled" : true,
            "_version" : "1.0.0",
            "testConnectionStatus": {
                "lastTestedAt": 1697833149,
                "state" : {
                    "success" : {
                        "response" : "{}",
                        "requestPayload" : "{}"
                    }
                }
            },
            "createdAt": 1697833149,
            "updatedAt": 1697833149,
            "updated": false,
            "version": "1.0.0",
            "lastModifiedBy": "system",
            "deleted": false,
            "changeLog": {},
            "tags": [],
            "active": true,
            "deprecated": false,
            "isDefaultCrudMapping": false,
        });

        let model_config: ConnectionModelDefinition =
            serde_json::from_value(sample_config).expect("Failed to deserialize ModelConfig");

        assert_eq!(model_config.name, "webhook_endpoints");
        assert_eq!(model_config.action, http::Method::GET);
        assert_eq!(model_config.action_name, CrudAction::GetOne);
        let PlatformInfo::Api(platform_info) = model_config.platform_info;
        assert_eq!(platform_info.base_url, "https://api.stripe.com/v1");
        assert_eq!(platform_info.path, "webhook_endpoints");
        assert_eq!(
            platform_info.auth_method,
            AuthMethod::BearerToken {
                value: "stripe_secret_key".to_string()
            }
        );
        assert_eq!(platform_info.query_params, None);
        assert_eq!(platform_info.headers, None);
        if let Some(ExtractorConfig {
            pull_frequency,
            batch_size,
            cursor,
            limit,
            sleep_after_finish,
            update_config,
            enabled,
        }) = model_config.extractor_config
        {
            assert_eq!(pull_frequency, 5);
            assert_eq!(batch_size, 100);
            assert_eq!(sleep_after_finish, 86400);
            assert_eq!(
                limit,
                Some(LimitConfig {
                    param_name: "limit".to_string(),
                    location: ParameterLocation::QueryParameter
                })
            );
            assert_eq!(
                cursor,
                CursorConfig {
                    param_name: Some("starting_after".to_string()),
                    location: Some(ParameterLocation::QueryParameter),
                    format: Some("{cursor}".to_string()),
                    cursor_path: "_.body.id".to_string(),
                    data_path: "_.body.data".to_string(),
                    reset_on_end: true,
                    js_extractor_function: None
                }
            );

            assert_eq!(update_config, None);
            assert!(enabled);
        } else {
            panic!("Wrong api config type");
        }
    }
}
