use super::{create, delete, read, update, CrudHook, CrudRequest, ReadResponse};
use crate::server::{AppState, AppStores};
use axum::{
    routing::{patch, post},
    Router,
};
use chrono::Utc;
use integrationos_domain::{
    algebra::MongoStore,
    common::{
        api_model_config::{ApiModelConfig, Compute, Function, Lang},
        connection_oauth_definition::{
            ComputeRequest, ConnectionOAuthDefinition, Frontend, OAuthApiConfig, OAuthCompute,
        },
        event_access::EventAccess,
    },
    id::{prefix::IdPrefix, Id},
    record_metadata::RecordMetadata,
};
use moka::future::Cache;
use mongodb::bson::doc;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, sync::Arc};

pub fn get_router() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/",
            post(create::<CreateRequest, ConnectionOAuthDefinition>)
                .get(read::<CreateRequest, ConnectionOAuthDefinition>),
        )
        .route(
            "/:id",
            patch(update::<CreateRequest, ConnectionOAuthDefinition>)
                .delete(delete::<CreateRequest, ConnectionOAuthDefinition>),
        )
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateRequest {
    pub connection_platform: String,
    pub platform_redirect_uri: String,
    pub ios_redirect_uri: String,
    pub scopes: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub separator: Option<String>,
    pub init: RequestParams,
    pub refresh: RequestParams,
}

impl CrudHook<ConnectionOAuthDefinition> for CreateRequest {}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestParams {
    pub configuration: ApiModelConfig,
    #[serde(skip_serializing_if = "Option::is_none", default = "default_separator")]
    pub compute: Option<String>,
    pub response_compute: String,
}

fn default_separator() -> Option<String> {
    Some(" ".to_string())
}

impl CrudRequest for CreateRequest {
    type Output = ConnectionOAuthDefinition;
    type Error = ();

    fn into_public(self) -> Result<Self::Output, Self::Error> {
        Ok(Self::Output {
            id: Id::new(IdPrefix::ConnectionOAuthDefinition, Utc::now()),
            connection_platform: self.connection_platform,
            configuration: OAuthApiConfig {
                init: self.init.configuration,
                refresh: self.refresh.configuration,
            },
            compute: OAuthCompute {
                init: ComputeRequest {
                    response: Function(Compute {
                        entry: "compute".to_string(),
                        function: self.init.response_compute,
                        language: Lang::JavaScript,
                    }),
                    computation: self.init.compute.map(|compute| {
                        Function(Compute {
                            entry: "compute".to_string(),
                            function: compute,
                            language: Lang::JavaScript,
                        })
                    }),
                },
                refresh: ComputeRequest {
                    computation: self.refresh.compute.map(|compute| {
                        Function(Compute {
                            entry: "compute".to_string(),
                            function: compute,
                            language: Lang::JavaScript,
                        })
                    }),
                    response: Function(Compute {
                        entry: "compute".to_string(),
                        function: self.refresh.response_compute,
                        language: Lang::JavaScript,
                    }),
                },
            },
            frontend: Frontend {
                platform_redirect_uri: self.platform_redirect_uri,
                ios_redirect_uri: self.ios_redirect_uri,
                scopes: self.scopes,
                separator: self.separator,
            },
            record_metadata: Default::default(),
            hooks: Default::default(),
        })
    }

    fn into_with_event_access(self, _event_access: Arc<EventAccess>) -> Self::Output {
        unimplemented!()
    }

    fn update(self, record: &mut Self::Output) {
        record.connection_platform = self.connection_platform;
        record.configuration = OAuthApiConfig {
            init: self.init.configuration,
            refresh: self.refresh.configuration,
        };
        record.compute = OAuthCompute {
            init: ComputeRequest {
                computation: self.init.compute.map(|compute| {
                    Function(Compute {
                        entry: "compute".to_string(),
                        function: compute,
                        language: Lang::JavaScript,
                    })
                }),
                response: Function(Compute {
                    entry: "compute".to_string(),
                    function: self.init.response_compute,
                    language: Lang::JavaScript,
                }),
            },
            refresh: ComputeRequest {
                response: Function(Compute {
                    entry: "compute".to_string(),
                    function: self.refresh.response_compute,
                    language: Lang::JavaScript,
                }),
                computation: self.refresh.compute.map(|compute| {
                    Function(Compute {
                        entry: "compute".to_string(),
                        function: compute,
                        language: Lang::JavaScript,
                    })
                }),
            },
        };
        record.frontend = Frontend {
            platform_redirect_uri: self.platform_redirect_uri,
            ios_redirect_uri: self.ios_redirect_uri,
            scopes: self.scopes,
            separator: self.separator,
        };
        record.record_metadata.updated_at = Utc::now().timestamp_millis();
        record.record_metadata.updated = true;
    }

    fn get_store(stores: AppStores) -> MongoStore<Self::Output> {
        stores.oauth_config.clone()
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FrontendOauthConnectionDefinition {
    #[serde(rename = "_id")]
    pub id: String,
    pub connection_platform: String,
    pub frontend: Frontend,
    #[serde(flatten)]
    pub record_metadata: RecordMetadata,
}

impl CrudRequest for FrontendOauthConnectionDefinition {
    type Output = FrontendOauthConnectionDefinition;
    type Error = ();

    fn into_public(self) -> Result<Self::Output, Self::Error> {
        unimplemented!()
    }

    fn into_with_event_access(self, _: Arc<EventAccess>) -> Self::Output {
        unimplemented!()
    }

    fn update(self, _: &mut Self::Output) {
        unimplemented!()
    }

    fn get_store(stores: AppStores) -> MongoStore<Self::Output> {
        stores.frontend_oauth_config.clone()
    }

    fn get_cache(
        state: Arc<AppState>,
    ) -> Arc<Cache<Option<BTreeMap<String, String>>, Arc<ReadResponse<Self::Output>>>> {
        state.connection_oauth_definitions_cache.clone()
    }
}
