use crate::domain::{ResponseCrudToMap, ResponseCrudToMapBuilder, ResponseCrudToMapRequest};
use crate::{
    algebra::jsruntime::{self, JSRuntimeImpl, JSRuntimeImplBuilder},
    client::CallerClient,
    domain::{RequestCrud, ResponseCrud, UnifiedCache, UnifiedMetadata, UnifiedMetadataBuilder},
    utility::{match_route, template_route},
};
use bson::doc;
use chrono::Utc;
use futures::{
    future::{join_all, OptionFuture},
    join, FutureExt,
};
use handlebars::Handlebars;
use http::header::ToStrError;
use http::{HeaderMap, HeaderName, HeaderValue, Response, StatusCode};
use integrationos_cache::local::{
    ConnectionCache, ConnectionModelDefinitionDestinationCache, ConnectionModelSchemaCache,
    LocalCacheExt, SecretCache,
};
use integrationos_domain::{
    algebra::JsonExt,
    api_model_config::{ModelPaths, RequestModelPaths, ResponseModelPaths},
    connection_model_definition::{
        ConnectionModelDefinition, CrudAction, CrudMapping, PlatformInfo,
    },
    connection_model_schema::ConnectionModelSchema,
    database::DatabaseConfig,
    destination::{Action, Destination},
    environment::Environment,
    error::InternalError,
    hashed_secret::HashedSecret,
    id::{prefix::IdPrefix, Id},
    prelude::{MongoStore, TimedExt},
    ApplicationError, Connection, ErrorMeta, IntegrationOSError, Secret, SecretExt, Store,
};
use js_sandbox_ios::Script;
use mongodb::{
    options::{Collation, CollationStrength, FindOneOptions},
    Client,
};
use serde_json::{json, Number, Value};
use std::{cell::RefCell, collections::HashMap, str::FromStr, sync::Arc};
use tracing::{debug, error};

pub struct UnifiedResponse {
    pub response: Response<Value>,
    pub metadata: UnifiedMetadata,
}

pub struct SendToDestinationUnified {
    pub action: Action,
    pub passthrough: bool,
    pub headers: HeaderMap,
    pub query_params: HashMap<String, String>,
    pub body: Option<Value>,
}

#[derive(Clone)]
pub struct UnifiedDestination {
    pub connections_cache: ConnectionCache,
    pub connections_store: MongoStore<Connection>,
    pub connection_model_definitions_cache: ConnectionModelDefinitionDestinationCache,
    pub connection_model_definitions_store: MongoStore<ConnectionModelDefinition>,
    pub connection_model_schemas_cache: ConnectionModelSchemaCache,
    pub connection_model_schemas_store: MongoStore<ConnectionModelSchema>,
    pub secrets_client: Arc<dyn SecretExt + Sync + Send>,
    pub secrets_cache: SecretCache,
    pub http_client: reqwest::Client,
}

pub struct UnifiedCacheTTLs {
    pub connection_cache_ttl_secs: u64,
    pub connection_model_definition_cache_ttl_secs: u64,
    pub connection_model_schema_cache_ttl_secs: u64,
    pub secret_cache_ttl_secs: u64,
}

impl UnifiedDestination {
    pub async fn new(
        db_config: DatabaseConfig,
        cache_size: u64,
        secrets_client: Arc<dyn SecretExt + Sync + Send>,
        cache_ttls: UnifiedCacheTTLs,
    ) -> Result<Self, IntegrationOSError> {
        let http_client = reqwest::Client::new();
        let connections_cache =
            ConnectionCache::new(cache_size, cache_ttls.connection_cache_ttl_secs);
        let connection_model_definitions_cache = ConnectionModelDefinitionDestinationCache::new(
            cache_size,
            cache_ttls.connection_model_definition_cache_ttl_secs,
        );
        let connection_model_schemas_cache = ConnectionModelSchemaCache::new(
            cache_size,
            cache_ttls.connection_model_schema_cache_ttl_secs,
        );
        let secrets_cache = SecretCache::new(cache_size, cache_ttls.secret_cache_ttl_secs);

        let client = Client::with_uri_str(&db_config.control_db_url)
            .await
            .map_err(|e| {
                InternalError::connection_error(
                    &format!("Failed to create UnifiedDestination client: {e}"),
                    None,
                )
            })?;

        let db = client.database(&db_config.control_db_name);

        let connections_store = MongoStore::new(&db, &Store::Connections).await?;
        let connection_model_definitions_store =
            MongoStore::new(&db, &Store::ConnectionModelDefinitions).await?;
        let connection_model_schemas_store =
            MongoStore::new(&db, &Store::ConnectionModelSchemas).await?;

        Ok(Self {
            connections_cache,
            connections_store,
            connection_model_definitions_cache,
            connection_model_definitions_store,
            connection_model_schemas_cache,
            connection_model_schemas_store,
            secrets_client,
            secrets_cache,
            http_client,
        })
    }

    pub async fn get_connection_model_definition(
        &self,
        destination: &Destination,
    ) -> Result<Option<ConnectionModelDefinition>, IntegrationOSError> {
        match &destination.action {
            Action::Passthrough { method, path } => {
                let connection_model_definitions = self
                    .connection_model_definitions_store
                    .get_many(
                        Some(doc! {
                            "connectionPlatform": destination.platform.as_ref(),
                            "action": method.as_str(),
                            "supported": true
                        }),
                        None,
                        None,
                        None,
                        None,
                    )
                    .await?;

                let routes = connection_model_definitions
                    .iter()
                    .map(|c| match c.platform_info {
                        PlatformInfo::Api(ref c) => c.path.as_ref(),
                    });

                let matched_route = match_route(path, routes.clone()).map(|r| r.to_string());

                let mut connection_model_definitions = connection_model_definitions
                    .clone()
                    .into_iter()
                    .filter(|c| match c.platform_info {
                        PlatformInfo::Api(ref c) => matched_route
                            .as_ref()
                            .map_or(false, |mr| c.path.as_str() == mr),
                    });

                if let Some(connection_model_definition) = connection_model_definitions.next() {
                    if connection_model_definitions.next().is_some() {
                        error!("Multiple connection model definitions found for this path. Destination: {:?}, Routes: {:?}", destination, routes);

                        return Err(InternalError::invalid_argument(
                            "Multiple connection model definitions found for this path",
                            None,
                        ));
                    }

                    Ok(Some(connection_model_definition))
                } else {
                    Ok(None)
                }
            }
            Action::Unified { name, action, .. } => Ok(self
                .connection_model_definitions_store
                .collection
                .find_one(doc! {
                    "connectionPlatform": destination.platform.as_ref(),
                    "mapping.commonModelName": name.as_ref(),
                    "actionName": action.to_string()
                })
                .with_options(
                    FindOneOptions::builder()
                        .collation(Some(
                            Collation::builder()
                                .strength(CollationStrength::Secondary)
                                .locale("en")
                                .build(),
                        ))
                        .build(),
                )
                .await?),
        }
    }

    pub async fn execute_model_definition_from_request(
        &self,
        config: &ConnectionModelDefinition,
        params: &RequestCrud,
        secret: &Value,
    ) -> Result<reqwest::Response, IntegrationOSError> {
        let context = match params.get_body() {
            None | Some(Value::Null) => None,
            _ => Some(serde_json::to_vec(&params.get_body()).map_err(|e| {
                error!(
                    "Failed to convert body to vec. ID: {}, Error: {}",
                    config.id, e
                );

                ApplicationError::bad_request(&e.to_string(), None)
            })?),
        };

        self.execute_model_definition(
            config,
            params.get_headers().to_owned(),
            params.get_query_params(),
            secret,
            context,
        )
        .await
    }

    pub async fn execute_model_definition(
        &self,
        config: &ConnectionModelDefinition,
        headers: HeaderMap,
        query_params: &HashMap<String, String>,
        secret: &Value,
        context: Option<Vec<u8>>,
    ) -> Result<reqwest::Response, IntegrationOSError> {
        let renderer = Handlebars::new();

        let config_str = serde_json::to_string(&config)
            .map_err(|e| InternalError::invalid_argument(&e.to_string(), None))?;

        let config = renderer
            .render_template(&config_str, secret)
            .map_err(|e| InternalError::invalid_argument(&e.to_string(), None))?;

        let config: ConnectionModelDefinition = serde_json::from_str(&config)
            .map_err(|e| InternalError::invalid_argument(&e.to_string(), None))?;

        match config.platform_info {
            PlatformInfo::Api(ref c) => {
                let api_caller = CallerClient::new(c, config.action, &self.http_client);

                let response = api_caller
                    .make_request(context, Some(secret), Some(headers), Some(query_params))
                    .await?;

                Ok(response)
            }
        }
    }

    pub async fn dispatch_unified_request(
        &self,
        connection: Arc<Connection>,
        action: Action,
        environment: Environment,
        params: RequestCrud,
    ) -> Result<UnifiedResponse, IntegrationOSError> {
        let mut metadata = UnifiedMetadataBuilder::default();
        let metadata = metadata
            .timestamp(Utc::now().timestamp_millis())
            .platform_rate_limit_remaining(0)
            .rate_limit_remaining(0)
            .host(params.get_header("host"))
            .transaction_key(Id::now(IdPrefix::Transaction))
            .platform(connection.platform.to_string())
            .platform_version(connection.platform_version.to_string())
            .common_model_version("v1")
            .connection_key(connection.key.to_string());

        self.perform_unified_request(connection, action, environment, params, metadata)
            .await
            .map_err(|e| match metadata.build().ok() {
                Some(metadata) => e.set_meta(&metadata.as_value()),
                None => e,
            })
    }

    async fn perform_unified_request(
        &self,
        connection: Arc<Connection>,
        action: Action,
        environment: Environment,
        params: RequestCrud,
        metadata: &mut UnifiedMetadataBuilder,
    ) -> Result<UnifiedResponse, IntegrationOSError> {
        let key = Destination {
            platform: connection.platform.clone(),
            action: action.clone(),
            connection_key: connection.key.clone(),
        };

        match action {
            Action::Unified {
                name,
                action,
                id,
                passthrough: is_passthrough,
            } => {
                // (ConnectionModelDefinition, Secret, ConnectionModelSchema)
                let (config, secret, cms) = self.get_dependencies(&key, &connection, &name).await.inspect_err(|e| {
                    error!("Failed to get dependencies for unified destination. Destination: {:?}, Error: {e}", key);
                })?;

                let metadata = metadata
                    .action(action.to_string())
                    .common_model(config.mapping.as_ref().map(|m| m.common_model_name.clone()).unwrap_or_default());

                let secret = insert_action_id(secret.as_value()?, id.as_ref());

                // Namespace for js scripts
                let crud_namespace = generate_script_namespace(self.secrets_cache.max_capacity(), &config.id.to_string());
                let schema_namespace = generate_script_namespace(self.secrets_cache.max_capacity(), &cms.id.to_string());


                let body = params.get_body().ok_or_else(|| InternalError::invalid_argument("No body found", None))?;
                let body = match cms.mapping.as_ref().map(|m| m.from_common_model.as_str()) {
                    Some(code) => {
                        let namespace = schema_namespace.clone() + "_mapFromCommonModel";

                        let jsruntime = JSRuntimeImplBuilder::default().namespace(namespace).code(code).build()
                            .inspect_err(|e| {
                                error!("Failed to create request schema mapping script for connection model. ID: {}, Error: {}", config.id, e);
                            })?;

                        jsruntime.create("mapFromCommonModel")?.run::<Value, Value>(body).await?.drop_nulls()
                    }
                    None => body.to_owned()
                };

                let default_params = params.clone();
                let request_crud: Option<Result<RequestCrud, IntegrationOSError>> = OptionFuture::from(config.mapping.as_ref().map(|m| m.from_common_model.to_owned())
                    .map(|code| async {
                        match code {
                            None => Ok(params),
                            Some(code) => {
                                let namespace = crud_namespace.clone() + "_mapFromCrudRequest";
                                let jsruntime = JSRuntimeImplBuilder::default().namespace(namespace).code(code).build()
                                    .inspect_err(|e| {
                                        error!("Failed to create request schema mapping script for connection model. ID: {}, Error: {}", config.id, e);
                                    })?;
                                let jsruntime = jsruntime.create("mapCrudRequest")?;

                                let params: RequestCrud = jsruntime.run(&prepare_crud_mapping(params, &config)?).await?;
                                let params: RequestCrud = params.extend_body(body);

                                Ok(params)
                            }
                        }
                    })).await;

                let params: RequestCrud = request_crud.unwrap_or(Ok(default_params))?;
                let secret: Value = extend_secret(secret, params.get_path_params());


                let body: Option<Value> = insert_body_into_path_object(&config, params.get_body());
                let params: RequestCrud = params.set_body(body);

                let response: reqwest::Response = self.execute_model_definition_from_request(&config, &params, &secret).await?;
                let status: StatusCode = response.status();
                let headers: HeaderMap = response.headers().clone();

                // convert to Result
                let error_for_status = if (response.status().is_client_error() || response.status().is_server_error()) {
                    Ok(())
                } else {
                    Err(InternalError::invalid_argument(&format!("Invalid response status: {}", status), None))
                };

                let model_definition_json: Result<Value, IntegrationOSError> = response.json().await.map_err(|e| {
                    error!("Failed to get json body from successful response. ID: {}, Error: {}", config.id, e);

                    IntegrationOSError::from_err_code(status, &e.to_string(), None)
                });

                let model_definition_json: Option<Value> = match error_for_status {
                    Err(e) => {
                        error!("Failed to execute model definition. ID: {}, Error: {}", config.id, e);

                        let mut response = Response::builder()
                            .status(status)
                            .body(model_definition_json?)
                            .map_err(|e| {
                                error!("Failed to create response from builder for unsuccessful response. ID: {}, Error: {}", config.id, e);

                                IntegrationOSError::from_err_code(status, &e.to_string(), None)
                            })?;
                        *response.headers_mut() = headers;
                        return Ok(UnifiedResponse { response, metadata: metadata.build()? });
                    }
                    Ok(_) => model_definition_json.ok(),
                };

                let passthrough: Option<Value> = if is_passthrough { model_definition_json.clone() } else { None };
                let pagination: Option<Value> = match &config.action_name {
                    CrudAction::GetMany => {
                        match cms.mapping.as_ref().map(|m| m.from_common_model.as_str()) {
                            Some(code) => {
                                let namespace = crud_namespace.clone() + "_mapToCrudRequest";
                                let jsruntime = JSRuntimeImplBuilder::default().namespace(namespace).code(code).build()
                                    .inspect_err(|e| {
                                        error!("Failed to create request crud mapping script for connection model. ID: {}, Error: {}", config.id, e);
                                    })?;
                                let jsruntime: JSRuntimeImpl = jsruntime.create("mapCrudRequest")?;

                                let pagination = extract_pagination(&config, &model_definition_json)?;
                                let res_to_map = ResponseCrudToMapBuilder::default()
                                    .headers(&headers)
                                    .pagination(pagination)
                                    .request(ResponseCrudToMapRequest::new(&params.get_query_params()))
                                    .build()?;

                                let response: ResponseCrud = jsruntime.run(&res_to_map).await?;

                                response.get_pagination().cloned()
                            }
                            _ => None
                        }
                    }
                    _ => None
                };
                // ** --> Progress

                todo!()
            }
            Action::Passthrough { method, path } => Err(InternalError::invalid_argument(
                &format!("Passthrough action is not supported for destination {}, in method {method} and path {path}", key.connection_key),
                None,
            )),
        }
    }

    // #[allow(clippy::too_many_arguments)]
    // pub async fn send_to_destination_unified(
    //     &self,
    //     connection: Arc<Connection>,
    //     action: Action,
    //     include_passthrough: bool,
    //     environment: Environment,
    //     mut headers: HeaderMap,
    //     mut query_params: HashMap<String, String>,
    //     mut body: Option<Value>,
    // ) -> Result<UnifiedResponse, IntegrationOSError> {
    //     let key = Destination {
    //         platform: connection.platform.clone(),
    //         action: action.clone(),
    //         connection_key: connection.key.clone(),
    //     };
    //
    //     let config_fut = self
    //         .connection_model_definitions_cache
    //         .get_or_insert_with_fn(key.clone(), || async {
    //             match self.get_connection_model_definition(&key).await {
    //                 Ok(Some(c)) => Ok(c),
    //                 Ok(None) => Err(InternalError::key_not_found("model definition", None)),
    //                 Err(e) => Err(InternalError::connection_error(
    //                     format!(
    //                         "Failed to get connection model definition: {}",
    //                         e.message().as_ref()
    //                     )
    //                         .as_str(),
    //                     None,
    //                 )),
    //             }
    //         });
    //
    //     let secret_fut =
    //         self.secrets_cache
    //             .get_or_insert_with_fn(connection.as_ref().clone(), || async {
    //                 match self
    //                     .secrets_client
    //                     .get(&connection.secrets_service_id, &connection.ownership.id)
    //                     .map(|v| Some(v).transpose())
    //                     .await
    //                 {
    //                     Ok(Some(c)) => Ok(c.as_value()?),
    //                     Ok(None) => Err(InternalError::key_not_found("secret", None)),
    //                     Err(e) => Err(InternalError::connection_error(
    //                         format!("Failed to get secret: {}", e.message().as_ref()).as_str(),
    //                         None,
    //                     )),
    //                 }
    //             });
    //
    //     let Action::Unified {
    //         action: _,
    //         id,
    //         name,
    //     } = &action
    //     else {
    //         return Err(InternalError::invalid_argument(
    //             "Sent a passthrough to the unified send to destination",
    //             None,
    //         ));
    //     };
    //
    //     let schema_key = (connection.platform.clone(), name.clone());
    //
    //     let schema_fut = self
    //         .connection_model_schemas_cache
    //         .get_or_insert_with_filter(
    //             &schema_key,
    //             self.connection_model_schemas_store.clone(),
    //             doc! {
    //                 "connectionPlatform": connection.platform.as_ref(),
    //                 "mapping.commonModelName": name.as_ref(),
    //             },
    //             Some(
    //                 FindOneOptions::builder()
    //                     .collation(Some(
    //                         Collation::builder()
    //                             .strength(CollationStrength::Secondary)
    //                             .locale("en")
    //                             .build(),
    //                     ))
    //                     .build(),
    //             ),
    //         );
    //
    //     tracing::debug!("Joining futures for {schema_key:?}");
    //
    //     let join_result = join!(config_fut, secret_fut, schema_fut);
    //
    //     let config = join_result.0.map_err(|e| {
    //         error!("Could not find connection model definition for destination with cache key {:?}: {:?}", key, e);
    //
    //         InternalError::key_not_found("model definition", None)
    //     })?;
    //     tracing::debug!(
    //         "Connection model definition found for destination with cache key {:?}",
    //         key
    //     );
    //
    //     let mut secret = join_result.1.map_err(|e| {
    //         error!(
    //             "Error getting secret for destination with cache key {:?}: {e}",
    //             key
    //         );
    //         InternalError::key_not_found(e.to_string().as_str(), None)
    //     })?;
    //
    //     tracing::debug!("Secret found for destination with cache key {:?}", key);
    //
    //     let cms = join_result.2.map_err(|e| {
    //         InternalError::key_not_found(&format!("model schema {name} for destination: {e}"), None)
    //     })?;
    //
    //     tracing::debug!(
    //         "Connection model schema found for destination with cache key {:?}",
    //         key
    //     );
    //
    //     let ConnectionModelSchema {
    //         id: schema_id,
    //         mapping,
    //         ..
    //     } = cms;
    //
    //     if let Some(id) = id {
    //         let secret = &mut secret;
    //         if let Value::Object(sec) = secret {
    //             const ID: &str = "id";
    //             sec.insert(ID.to_string(), Value::String(id.to_string()));
    //         }
    //     }
    //
    //     let crud_script_namespace = if self.secrets_cache.max_capacity() == 0 {
    //         "$".to_string() + &uuid::Uuid::new_v4().simple().to_string()
    //     } else {
    //         config.id.to_string().replace([':', '-'], "_")
    //     };
    //     let schema_script_namespace = if self.secrets_cache.max_capacity() == 0 {
    //         "$".to_string() + &uuid::Uuid::new_v4().simple().to_string()
    //     } else {
    //         schema_id.to_string().replace([':', '-'], "_")
    //     };
    //
    //     let mut metadata = json!({
    //         "timestamp": Utc::now().timestamp_millis(),
    //         "platformRateLimitRemaining": 0,
    //         "rateLimitRemaining": 0,
    //         "host": headers.get("host").map(|v| v.to_str().unwrap_or("")),
    //         "cache": {
    //             "hit": false,
    //             "ttl": 0,
    //             "key": ""
    //         },
    //         "transactionKey": Id::now(IdPrefix::Transaction),
    //         "platform": connection.platform,
    //         "platformVersion": connection.platform_version,
    //         "action": config.action_name,
    //         "commonModel": config.mapping.as_ref().map(|m| &m.common_model_name),
    //         "commonModelVersion": "v1",
    //         "connectionKey": connection.key,
    //     });
    //
    //     **Again this is referencing the model_definition_json not the body from the request --> Progress
    //     body = if let Some(body) = body {
    //         if let Some(js) = mapping.as_ref().map(|m| m.from_common_model.as_str()) {
    //             debug!(
    //                 "Mapping request body {}\nUsing js {js}",
    //                 serde_json::to_string_pretty(&body)
    //                     .map_err(|e| {
    //                         error!("Failed to convert body to pretty string for connection model. ID: {}, Body: {}, Error: {}", config.id, body, e);
    //                     })
    //                     .unwrap_or_default(),
    //             );
    //
    //             let ns: String = schema_script_namespace.clone() + "_mapFromCommonModel";
    //             JS_RUNTIME
    //                 .with_borrow_mut(|script| script.add_script(&ns, "mapFromCommonModel", js))
    //                 .map_err(|e| {
    //                     error!("Failed to create request schema mapping script for connection model. ID: {}, Error: {}", config.id, e);
    //
    //                     ApplicationError::bad_request(
    //                         &format!("Failed while creating request schema mapping script: {e}"),
    //                         None,
    //                     )
    //                         .set_meta(&metadata)
    //                 })?;
    //             let body = JS_RUNTIME
    //                 .with_borrow_mut(|script| script.call_namespace(&ns, body))
    //                 .map_err(|e| {
    //                     error!("Failed to run request schema mapping script for connection model. ID: {}, Error: {}", config.id, e);
    //
    //                     ApplicationError::bad_request(
    //                         &format!("Failed while running request schema mapping script: {e}"),
    //                         None,
    //                     )
    //                         .set_meta(&metadata)
    //                 })?;
    //
    //             tokio::task::yield_now().await;
    //
    //             let body = remove_nulls(&body);
    //
    //             debug!(
    //                 "Mapped body to {}",
    //                 serde_json::to_string_pretty(&body)
    //                     .map_err(|e| {
    //                         error!("Failed to convert mapped body to pretty string. ID: {}, Body: {}, Error: {}", config.id, body, e);
    //                     })
    //                     .unwrap_or_default(),
    //             );
    //
    //             Some(body)
    //         } else {
    //             debug!(
    //                 "No js for schema mapping to common model {name} for {}",
    //                 connection.platform
    //             );
    //             Some(body)
    //         }
    //     } else {
    //         debug!("No body to map");
    //         None
    //     };
    //
    //     if let Some(CrudMapping {
    //                     from_common_model: Some(js),
    //                     ..
    //                 }) = &config.mapping
    //     {
    //         if !js.is_empty() {
    //             let ns: String = crud_script_namespace.clone() + "_mapFromCrudRequest";
    //             JS_RUNTIME
    //                 .with_borrow_mut(|script| script.add_script(&ns, "mapCrudRequest", js.as_str()))
    //                 .map_err(|e| {
    //                     error!("Failed to create request crud mapping script for connection model. ID: {}, JS: {}, Error: {}", config.id, js, e);
    //                     ApplicationError::bad_request(&e.to_string(), None).set_meta(&metadata)
    //                 })?;
    //
    //             const PASSTHROUGH_PARAMS: &str = "passthroughForward";
    //             const PASSHTROUGH_HEADERS: &str = "x-integrationos-passthrough-forward";
    //
    //             if let Some(custom_params) = query_params.remove(PASSTHROUGH_PARAMS) {
    //                 let pairs = custom_params.split('&').filter_map(|pair| {
    //                     pair.split_once('=')
    //                         .map(|(a, b)| (a.to_owned(), b.to_owned()))
    //                 });
    //                 query_params.extend(pairs);
    //             }
    //
    //             if let Some(custom_headers) = headers.remove(PASSHTROUGH_HEADERS) {
    //                 let pairs = custom_headers
    //                     .to_str()
    //                     .map_err(|e| {
    //                         error!(
    //                             "Failed to convert custom headers to string. ID {:?}, Error: {:?}",
    //                             config.id, e
    //                         );
    //                         InternalError::invalid_argument(&e.to_string(), None)
    //                             .set_meta(&metadata)
    //                     })?
    //                     .split(';')
    //                     .filter_map(|pair| pair.split_once('='))
    //                     .filter_map(|(a, b)| {
    //                         match (HeaderName::from_str(a).ok(), HeaderValue::try_from(b).ok()) {
    //                             (Some(a), Some(b)) => Some((Some(a), b)),
    //                             _ => None,
    //                         }
    //                     });
    //                 headers.extend(pairs);
    //             }
    //
    //             let request = RequestCrudBorrowed {
    //                 query_params: &query_params,
    //                 headers: &headers,
    //                 path_params: id.as_ref().map(|id| PathParams { id }),
    //             };
    //
    //             debug!(
    //                 "Mapping request crud {}\nUsing js {js}",
    //                 serde_json::to_string_pretty(&request)
    //                     .map_err(|e| {
    //                         error!("Failed to convert request crud to pretty string. ID: {}, Request: {:?}, Error: {}", config.id, request, e);
    //                     })
    //                     .unwrap_or_default(),
    //             );
    //
    //             let res: RequestCrud = JS_RUNTIME
    //                 .with_borrow_mut(|script| script.call_namespace(&ns, request))
    //                 .map_err(|e| {
    //                     error!("Failed to run request crud mapping script for connection model. ID: {}, Error: {}", config.id, e);
    //
    //                     ApplicationError::bad_request(
    //                         &format!("Failed while running request crud mapping script: {e}"),
    //                         None,
    //                     )
    //                         .set_meta(&metadata)
    //                 })?;
    //
    //             debug!(
    //                 "Mapped request crud to {}",
    //                 serde_json::to_string_pretty(&res)
    //                     .map_err(|e| {
    //                         error!("Failed to convert crud to pretty string. ID: {}, Res: {:?}, Error: {}", config.id, res, e);
    //                     })
    //                     .unwrap_or_default(),
    //             );
    //
    //             headers = res.headers;
    //
    //             query_params = res.query_params.unwrap_or_default();
    //
    //             let secret = &mut secret;
    //             if let Value::Object(ref mut sec) = secret {
    //                 if let Some(path_params) = res.path_params {
    //                     sec.extend(path_params.into_iter().map(|(a, b)| (a, Value::String(b))));
    //                 }
    //             }
    //
    //             match (&mut body, res.body) {
    //                 (Some(Value::Object(a)), Some(Value::Object(b))) => {
    //                     a.extend(b);
    //                 }
    //                 (body @ None, Some(mapped_body)) => {
    //                     body.replace(mapped_body);
    //                 }
    //                 _ => {}
    //             }
    //         }
    //     }
    //
    //     let PlatformInfo::Api(api_config) = &config.platform_info;
    //
    //     if let Some(ModelPaths {
    //                     request: Some(RequestModelPaths { object: Some(path) }),
    //                     ..
    //                 }) = &api_config.paths
    //     {
    //         if let Some(path) = path.strip_prefix("$.body.") {
    //             body = body.map(|body| json!({path: body}));
    //             debug!(
    //                 "Mapped request body to {path}: {}",
    //                 serde_json::to_string_pretty(&body)
    //                     .map_err(|e| {
    //                         error!("Failed to convert mapped body to pretty string. ID: {}, Body: {:?}, Error: {}", config.id, body, e);
    //                     })
    //                     .unwrap_or_default(),
    //             );
    //         }
    //     }
    //
    //     debug!("Executing model definition with config {config:#?}, headers {headers:#?}, query params {query_params:#?}");
    //
    //     let context = match body {
    //         None | Some(Value::Null) => None,
    //         _ => Some(serde_json::to_vec(&body).map_err(|e| {
    //             error!(
    //                 "Failed to convert body to vec. ID: {}, Error: {}",
    //                 config.id, e
    //             );
    //
    //             ApplicationError::bad_request(&e.to_string(), None).set_meta(&metadata)
    //         })?),
    //     };
    //
    //     let mut latency = 0i64;
    //     let mut res = self
    //         .execute_model_definition(&config, headers, &query_params, &secret, context)
    //         .timed(|_, duration| {
    //             latency = duration.as_millis() as i64;
    //         })
    //         .await
    //         .map_err(|e| {
    //             error!(
    //                 "Failed to execute connection model definition. ID: {}, Error: {:?}",
    //                 config.id, e
    //             );
    //             e.set_meta(&metadata)
    //         })?;
    //
    //     debug!(
    //         "Executed model definition with status code {}, headers: {:#?}",
    //         res.status(),
    //         res.headers()
    //     );
    //
    //     let headers = std::mem::take(res.headers_mut());
    //
    //     if !res.status().is_success() {
    //         let status = res.status();
    //
    //         let mut res = Response::builder()
    //             .status(status)
    //             .body(res.json().await.map_err(|e| {
    //                 error!("Failed to get json body from unsuccessful response. ID: {}, Error: {}", config.id, e);
    //
    //                 IntegrationOSError::from_err_code(status, &e.to_string(), None)
    //                     .set_meta(&metadata)
    //             })?)
    //             .map_err(|e| {
    //                 error!("Failed to create response from builder for unsuccessful response. ID: {}, Error: {}", config.id, e);
    //
    //                 IntegrationOSError::from_err_code(status, &e.to_string(), None)
    //                     .set_meta(&metadata)
    //             })?;
    //         *res.headers_mut() = headers;
    //         return Ok(UnifiedResponse {
    //             metadata: metadata.clone(),
    //             response: res,
    //         });
    //     }
    //
    //     let status = res.status();
    //     // From this point on anything related to body is no longer the body that came in the
    //     // function but the body that was returned by the destination
    //
    //     let mut body: Option<Value> = res.json().await.ok();
    //
    //     let passthrough = if include_passthrough {
    //         body.clone()
    //     } else {
    //         None
    //     };
    //
    //     debug!(
    //         "Received response body: {}",
    //         serde_json::to_string_pretty(&body)
    //             .map_err(|e| {
    //                 error!(
    //                     "Failed to convert body to pretty string. ID: {}, Body: {:?}, Error: {} ",
    //                     config.id, body, e
    //                 );
    //             })
    //             .unwrap_or_default(),
    //     );
    //
    //     let pagination = if config.action_name == CrudAction::GetMany {
    //         if let Some(CrudMapping {
    //                         to_common_model: Some(js),
    //                         ..
    //                     }) = &config.mapping
    //         {
    //             if !js.is_empty() {
    //                 let ns: String = crud_script_namespace + "_mapToCrudRequest";
    //                 JS_RUNTIME
    //                     .with_borrow_mut(|script| {
    //                         script.add_script(&ns, "mapCrudRequest", js.as_str())
    //                     })
    //                     .map_err(|e| {
    //                         error!("Failed to create response crud mapping script for connection model. ID: {}, JS: {}, Error: {}", config.id, js, e);
    //
    //                         ApplicationError::bad_request(&e.to_string(), None).set_meta(&metadata)
    //                     })?;
    //
    //                 let pagination = if let (
    //                     Some(ModelPaths {
    //                              response:
    //                              Some(ResponseModelPaths {
    //                                       cursor: Some(path), ..
    //                                   }),
    //                              ..
    //                          }),
    //                     Some(body),
    //                 ) = (&api_config.paths, &body)
    //                 {
    //                     let wrapped_body = json!({"body":body});
    //                     let mut bodies =
    //                         jsonpath_lib::select(&wrapped_body, path).map_err(|e| {
    //                             error!("Failed to select cursor at response path. ID: {}, Path: {}, Error: {}", config.id, path, e);
    //
    //                             ApplicationError::bad_request(&e.to_string(), None)
    //                                 .set_meta(&metadata)
    //                         })?;
    //                     if bodies.len() != 1 {
    //                         Some(Value::Null)
    //                     } else {
    //                         Some(bodies.remove(0).clone())
    //                     }
    //                 } else {
    //                     None
    //                 };
    //
    //                 let res_to_map = ResponseCrudToMap {
    //                     headers: &headers,
    //                     pagination,
    //                     request: ResponseCrudToMapRequest {
    //                         query_params: &query_params,
    //                     },
    //                 };
    //
    //                 debug!(
    //                     "Mapping response crud {}\nUsing js {js}",
    //                     serde_json::to_string_pretty(&res_to_map).map_err(|e| {
    //                         error!("Failed to convert response crud to pretty string. ID: {}, Response to Map: {:?}, Error: {}", config.id, res_to_map, e);
    //                     })
    //                     .unwrap_or_default(),
    //                 );
    //
    //                 let res: ResponseCrud = JS_RUNTIME
    //                     .with_borrow_mut(|script| script.call_namespace(&ns, &res_to_map))
    //                     .map_err(|e| {
    //                         ApplicationError::bad_request(
    //                             &format!("Failed while running response crud mapping script. ID: {}, Error: {}", config.id, e),
    //                             None,
    //                         )
    //                             .set_meta(&metadata)
    //                     })?;
    //
    //                 tokio::task::yield_now().await;
    //
    //                 debug!(
    //                     "Mapped response crud to {}",
    //                     serde_json::to_string_pretty(&res).map_err(|e| {
    //                         error!("Failed to convert response crud to pretty string. ID: {}, Response: {:?}, Error: {}", config.id, res, e);
    //
    //                         InternalError::invalid_argument(&e.to_string(), None)
    //                             .set_meta(&metadata)
    //                     })?
    //                 );
    //
    //                 res.pagination
    //             } else {
    //                 None
    //             }
    //         } else {
    //             None
    //         }
    //     } else {
    //         None
    //     };
    //
    //     if let Some(ModelPaths {
    //                     response:
    //                     Some(ResponseModelPaths {
    //                              object: Some(path), ..
    //                          }),
    //                     ..
    //                 }) = &api_config.paths
    //     {
    //         body = if let Some(body) = body {
    //             let wrapped_body = json!({"body":body});
    //             let mut bodies = jsonpath_lib::select(&wrapped_body, path).map_err(|e| {
    //                 error!(
    //                     "Failed to select body at response path. ID {}, Path {}, Error {}",
    //                     config.id, path, e
    //                 );
    //
    //                 ApplicationError::bad_request(&e.to_string(), None).set_meta(&metadata)
    //             })?;
    //
    //             let is_returning_error = !environment.is_production()
    //                 && matches!(config.action_name, CrudAction::GetMany | CrudAction::GetOne);
    //             let is_parseable_body = !bodies.is_empty() && bodies.len() == 1;
    //
    //             if bodies.is_empty() && is_returning_error {
    //                 let error_string = format!(
    //                     "Could not map unified model. 3rd party Connection returned an invalid response. Expected model at path {path} but found none.",
    //                 );
    //                 let mut res = Response::builder()
    //                     .status(StatusCode::UNPROCESSABLE_ENTITY)
    //                     .body(json!({
    //                         "message": error_string,
    //                         "passthrough": wrapped_body
    //                     }))
    //                     .map_err(|e| {
    //                         error!("Failed to create response from builder for missing body. ID: {}, Error: {}", config.id, e);
    //
    //                         IntegrationOSError::from_err_code(
    //                             StatusCode::UNPROCESSABLE_ENTITY,
    //                             &e.to_string(),
    //                             None,
    //                         )
    //                             .set_meta(&metadata)
    //                     })?;
    //                 *res.headers_mut() = headers;
    //                 return Ok(UnifiedResponse {
    //                     metadata: metadata.clone(),
    //                     response: res,
    //                 });
    //             }
    //
    //             if bodies.len() != 1 && is_returning_error {
    //                 return Err(InternalError::invalid_argument(
    //                     &format!(
    //                         "Invalid number of selected bodies ({}) at response path {} for CMD with ID: {}",
    //                         bodies.len(),
    //                         path,
    //                         config.id
    //                     ),
    //                     None,
    //                 )
    //                     .set_meta(&metadata));
    //             }
    //
    //             if is_parseable_body {
    //                 Some(bodies.remove(0).clone())
    //             } else {
    //                 None
    //             }
    //         } else {
    //             None
    //         };
    //         debug!(
    //             "Mapped response body to {path}: {}",
    //             serde_json::to_string_pretty(&body)
    //                 .map_err(|e| {
    //                     error!("Could not convert mapped body to pretty string {body:?}: {e}");
    //                 })
    //                 .unwrap_or_default(),
    //         );
    //     }
    //
    //     if matches!(
    //         config.action_name,
    //         CrudAction::GetMany | CrudAction::GetOne | CrudAction::Create | CrudAction::Upsert
    //     ) {
    //         let Some(js) = mapping.as_ref().map(|m| &m.to_common_model) else {
    //             return Err(InternalError::invalid_argument(
    //                 &format!(
    //                     "No js for schema mapping to common model {name} for {}. ID: {}",
    //                     connection.platform, config.id
    //                 ),
    //                 None,
    //             )
    //                 .set_meta(&metadata));
    //         };
    //         let ns: String = schema_script_namespace + "_mapToCommonModel";
    //         JS_RUNTIME
    //             .with_borrow_mut(|script| script.add_script(&ns, "mapToCommonModel", js))
    //             .map_err(|e| {
    //                 error!("Failed to create response schema mapping script for connection model. ID: {}, JS: {}, Error: {}", config.id, js, e);
    //
    //                 ApplicationError::bad_request(&e.to_string(), None).set_meta(&metadata)
    //             })?;
    //
    //         debug!(
    //             "Mapping response body {}\nUsing js {js}",
    //             serde_json::to_string_pretty(&body)
    //                 .map_err(|e| {
    //                     error!("Could not convert body to pretty string {body:?}: {e}");
    //                 })
    //                 .unwrap_or_default(),
    //         );
    //
    //         const ID_KEY: &str = "id";
    //         const MODIFY_TOKEN_KEY: &str = "modifyToken";
    //
    //         let mapped_body: Value = if let Some(Value::Array(arr)) = body {
    //             let mut futs = Vec::with_capacity(arr.len());
    //             for body in arr {
    //                 futs.push(async {
    //                     let res =
    //                         JS_RUNTIME.with_borrow_mut(|script| {
    //                             script
    //                                 .add_script(&ns, "mapToCommonModel", js)
    //                                 .and_then(|_| script.call_namespace(&ns, body))
    //                                 .map_err(|e| {
    //                                     ApplicationError::bad_request(
    //                                         &format!("Failed while running response schema mapping script: {}. ID: {}", e, config.id),
    //                                         None,
    //                                     )
    //                                         .set_meta(&metadata)
    //                                 })
    //                         });
    //                     tokio::task::yield_now().await;
    //                     res.map(|mut body| {
    //                         if let Value::Object(map) = &mut body {
    //                             if !map.contains_key(MODIFY_TOKEN_KEY) {
    //                                 let v = map.get(ID_KEY).cloned().unwrap_or(json!(""));
    //                                 map.insert(MODIFY_TOKEN_KEY.to_owned(), v);
    //                             }
    //                         }
    //                         body
    //                     })
    //                 });
    //             }
    //             let values = join_all(futs)
    //                 .await
    //                 .into_iter()
    //                 .collect::<Result<Vec<Value>, _>>()?;
    //             Value::Array(values)
    //         } else if let Some(body) = &body {
    //             JS_RUNTIME
    //                 .with_borrow_mut(|script| script.call_namespace(&ns, body))
    //                 .map(|mut body| {
    //                     if let Value::Object(map) = &mut body {
    //                         if !map.contains_key(MODIFY_TOKEN_KEY) {
    //                             let v = map.get(ID_KEY).cloned().unwrap_or(json!(""));
    //                             map.insert(MODIFY_TOKEN_KEY.to_owned(), v);
    //                         }
    //                     }
    //                     body
    //                 })
    //                 .map_err(|e| {
    //                     ApplicationError::bad_request(
    //                         &format!("Failed while running response schema mapping script. ID: {}, Error: {}", config.id, e),
    //                         None,
    //                     )
    //                         .set_meta(&metadata)
    //                 })?
    //         } else if matches!(config.action_name, CrudAction::GetMany) {
    //             Value::Array(Default::default())
    //         } else {
    //             Value::Object(Default::default())
    //         };
    //
    //         let mapped_body = remove_nulls(&mapped_body);
    //
    //         body = Some(mapped_body);
    //     } else if matches!(config.action_name, CrudAction::Update | CrudAction::Delete) {
    //         body = None;
    //     }
    //
    //     debug!(
    //         "Mapped response body to {}",
    //         serde_json::to_string_pretty(&body)
    //             .map_err(|e| {
    //                 error!("Could not convert body to pretty string {body:?}: {e}");
    //             })
    //             .unwrap_or_default(),
    //     );
    //
    //     let mut response = json!({});
    //
    //     let response_len = if let Some(Value::Array(arr)) = &body {
    //         arr.len()
    //     } else {
    //         0
    //     };
    //
    //     let hash = HashedSecret::try_from(json!({
    //         "response": &body,
    //         "action": config.action_name,
    //         "commonModel": config.mapping.as_ref().map(|m| &m.common_model_name),
    //     }))
    //         .map_err(|e| e.set_meta(&metadata))?;
    //
    //     match body {
    //         Some(body) => {
    //             const UNIFIED: &str = "unified";
    //             const COUNT: &str = "count";
    //
    //             match response {
    //                 Value::Object(ref mut response) => {
    //                     if config.action_name == CrudAction::GetCount {
    //                         response.insert(UNIFIED.to_string(), json!({ COUNT: body }));
    //                     } else {
    //                         response.insert(UNIFIED.to_string(), body);
    //                     }
    //                 }
    //                 Value::Number(ref mut count) => {
    //                     if config.action_name == CrudAction::GetCount {
    //                         response = json!({ UNIFIED: { COUNT: count } });
    //                     }
    //                 }
    //                 _ => {}
    //             }
    //         }
    //         None => tracing::info!(
    //             "There was no response body to map for this action. ID: {}",
    //             config.id
    //         ),
    //     };
    //
    //     if let (true, Some(passthrough), Value::Object(ref mut response)) =
    //         (include_passthrough, passthrough, &mut response)
    //     {
    //         const PASSTHROUGH: &str = "passthrough";
    //         response.insert(PASSTHROUGH.to_string(), passthrough);
    //     }
    //
    //     if let (Some(Value::Object(mut pagination)), Value::Object(ref mut response)) =
    //         (pagination, &mut response)
    //     {
    //         const LIMIT: &str = "limit";
    //         if let Some(Ok(limit)) = query_params.get(LIMIT).map(|s| s.parse::<u32>()) {
    //             pagination.insert(LIMIT.to_string(), Value::Number(Number::from(limit)));
    //         }
    //         const PAGE_SIZE: &str = "pageSize";
    //         pagination.insert(
    //             PAGE_SIZE.to_string(),
    //             Value::Number(Number::from(response_len)),
    //         );
    //         const PAGINATION: &str = "pagination";
    //         response.insert(PAGINATION.to_string(), Value::Object(pagination));
    //     }
    //
    //     if let Value::Object(ref mut response) = &mut response {
    //         if let Some(meta) = metadata.as_object_mut() {
    //             meta.insert("latency".to_string(), Value::Number(Number::from(latency)));
    //             meta.insert("hash".to_string(), Value::String(hash.inner().into()));
    //         }
    //
    //         const META: &str = "meta";
    //         response.insert(META.to_string(), metadata.clone());
    //     }
    //
    //     let mut builder = Response::builder();
    //
    //     if status.is_success() {
    //         const STATUS_HEADER: &str = "response-status";
    //         builder = builder
    //             .header::<&'static str, HeaderValue>(STATUS_HEADER, status.as_u16().into())
    //             .status(StatusCode::OK);
    //     } else {
    //         builder = builder.status(status);
    //     }
    //     if let Some(builder_headers) = builder.headers_mut() {
    //         builder_headers.extend(headers.into_iter());
    //     } else {
    //         return Err(IntegrationOSError::from_err_code(
    //             status,
    //             "Could not get headers from builder",
    //             None,
    //         )
    //             .set_meta(&metadata));
    //     };
    //     let res = builder.body(response).map_err(|e| {
    //         error!(
    //             "Failed to create response from builder for successful response. ID: {}, Error: {}",
    //             config.id, e
    //         );
    //         IntegrationOSError::from_err_code(status, &e.to_string(), None).set_meta(&metadata)
    //     })?;
    //
    //     Ok(UnifiedResponse {
    //         metadata: metadata.clone(),
    //         response: res,
    //     })
    // }

    pub async fn dispatch_destination_request(
        &self,
        connection: Option<Arc<Connection>>,
        destination: &Destination,
        headers: HeaderMap,
        query_params: HashMap<String, String>,
        context: Option<Vec<u8>>,
    ) -> Result<reqwest::Response, IntegrationOSError> {
        let connection = if let Some(connection) = connection {
            connection
        } else {
            Arc::new(
                self.connections_cache
                    .get_or_insert_with_filter(
                        &destination.connection_key,
                        self.connections_store.clone(),
                        doc! { "key": destination.connection_key.as_ref() },
                        None,
                    )
                    .await?,
            )
        };

        let config = match self.get_connection_model_definition(destination).await {
            Ok(Some(c)) => Ok(Arc::new(c)),
            Ok(None) => Err(InternalError::key_not_found(
                "ConnectionModelDefinition",
                None,
            )),
            Err(e) => Err(InternalError::connection_error(
                format!(
                    "Failed to get connection model definition: {}",
                    e.message().as_ref()
                )
                .as_str(),
                None,
            )),
        }?;

        if !config.supported {
            return Err(ApplicationError::not_found(
                "Supported Connection Model Definition",
                None,
            ));
        }

        let secret = self
            .secrets_cache
            .get_or_insert_with_fn(connection.as_ref(), || async {
                match self
                    .secrets_client
                    .get(&connection.secrets_service_id, &connection.ownership.id)
                    .map(|v| Some(v).transpose())
                    .await
                {
                    Ok(Some(c)) => Ok(c),
                    Ok(None) => Err(InternalError::key_not_found("Secrets", None)),
                    Err(e) => Err(InternalError::connection_error(
                        format!("Failed to get secret: {}", e.message().as_ref()).as_str(),
                        None,
                    )),
                }
            })
            .await?;

        // Template the route for passthrough actions
        let templated_config = match &destination.action {
            Action::Passthrough { method: _, path } => {
                let mut config_clone = (*config).clone();
                let PlatformInfo::Api(ref mut c) = config_clone.platform_info;
                let template = template_route(c.path.clone(), path.to_string());
                c.path = template;
                config_clone.platform_info = PlatformInfo::Api(c.clone());
                Arc::new(config_clone)
            }
            _ => config.clone(),
        };

        self.execute_model_definition(
            &templated_config,
            headers,
            &query_params,
            &secret.as_value()?,
            context,
        )
        .await
    }

    async fn get_dependencies(
        &self,
        key: &Destination,
        connection: &Connection,
        name: &str,
    ) -> Result<(ConnectionModelDefinition, Secret, ConnectionModelSchema), IntegrationOSError>
    {
        let config_fut = self
            .connection_model_definitions_cache
            .get_or_insert_with_fn(key, || async {
                match self.get_connection_model_definition(key).await {
                    Ok(Some(c)) => Ok(c),
                    Ok(None) => Err(InternalError::key_not_found("model definition", None)),
                    Err(e) => Err(InternalError::connection_error(
                        format!(
                            "Failed to get connection model definition: {}",
                            e.message().as_ref()
                        )
                        .as_str(),
                        None,
                    )),
                }
            });

        let secret_fut = self
            .secrets_cache
            .get_or_insert_with_fn(connection, || async {
                match self
                    .secrets_client
                    .get(&connection.secrets_service_id, &connection.ownership.id)
                    .map(|v| Some(v).transpose())
                    .await
                {
                    Ok(Some(c)) => Ok(c),
                    Ok(None) => Err(InternalError::key_not_found("secret", None)),
                    Err(e) => Err(InternalError::connection_error(
                        format!("Failed to get secret: {}", e.message().as_ref()).as_str(),
                        None,
                    )),
                }
            });

        let schema_key: (Arc<str>, Arc<str>) = (connection.platform.clone(), name.into());

        let schema_fut = self
            .connection_model_schemas_cache
            .get_or_insert_with_filter(
                &schema_key,
                self.connection_model_schemas_store.clone(),
                doc! {
                    "connectionPlatform": connection.platform.as_ref(),
                    "mapping.commonModelName": name,
                },
                Some(
                    FindOneOptions::builder()
                        .collation(Some(
                            Collation::builder()
                                .strength(CollationStrength::Secondary)
                                .locale("en")
                                .build(),
                        ))
                        .build(),
                ),
            );

        let res = tokio::join!(config_fut, secret_fut, schema_fut);

        match res {
            (Ok(c), Ok(s), Ok(m)) => Ok((c, s, m)),
            (Err(e), _, _) => Err(e),
            (_, Err(e), _) => Err(e),
            (_, _, Err(e)) => Err(e),
        }
    }
}

fn extract_pagination(
    config: &ConnectionModelDefinition,
    body: &Option<Value>,
) -> Result<Option<Value>, IntegrationOSError> {
    let path = match config
        .platform_info
        .config()
        .paths
        .as_ref()
        .and_then(|paths| paths.response.as_ref())
        .and_then(|response| response.cursor.as_ref())
    {
        Some(p) => p,
        None => return Ok(None),
    };

    let body_value = match body {
        Some(b) => b,
        None => return Ok(None),
    };

    let wrapped_body = json!({ "body": body_value });

    let mut bodies = jsonpath_lib::select(&wrapped_body, path).map_err(|e| {
        error!(
            "Failed to select cursor at response path. ID: {}, Path: {}, Error: {}",
            config.id, path, e
        );
        ApplicationError::bad_request(&e.to_string(), None)
    })?;

    Ok(if bodies.len() == 1 {
        Some(bodies.remove(0).clone())
    } else {
        Some(Value::Null)
    })
}

fn insert_body_into_path_object(
    config: &ConnectionModelDefinition,
    body: Option<&Value>,
) -> Option<Value> {
    match config.platform_info.config().paths.as_ref() {
        Some(ModelPaths {
            request: Some(RequestModelPaths { object: Some(path) }),
            ..
        }) => {
            if let Some(path) = path.strip_prefix("$.body.") {
                body.map(|body| json!({ path: body }))
            } else {
                body.cloned()
            }
        }
        _ => body.cloned(),
    }
}

fn extend_secret(mut secret: Value, get_path_params: Option<&HashMap<String, String>>) -> Value {
    if let Value::Object(sec) = &mut secret {
        if let Some(path_params) = get_path_params {
            sec.extend(
                path_params
                    .iter()
                    .map(|(lhs, rhs)| (lhs.to_string(), Value::String(rhs.to_string()))),
            );
        }
    }

    secret
}

fn generate_script_namespace(max_capacity: u64, key: &str) -> String {
    if max_capacity == 0 {
        "$".to_string() + &uuid::Uuid::new_v4().simple().to_string()
    } else {
        key.to_string().replace([':', '-'], "_")
    }
}

fn insert_action_id(secret: Value, id: Option<&Arc<str>>) -> Value {
    if let Value::Object(mut sec) = secret {
        if let Some(id) = id {
            sec.insert("id".to_string(), Value::String(id.to_string()));
        }
        Value::Object(sec)
    } else {
        secret
    }
}

/// Prepares the CRUD (Create, Read, Update, Delete) mapping by modifying the request's
/// query parameters and headers based on user-defined and connection-specific configurations.
///
/// # Arguments
///
/// * `params` - A `RequestCrud` object containing the initial request parameters and headers.
/// * `config` - A reference to a `ConnectionModelDefinition` object that provides the connection-specific configurations.
///
/// # Returns
///
/// Returns a `Result` containing either:
/// - An updated `RequestCrud` object with modified query parameters and headers.
/// - An `IntegrationOSError` if an error occurs during processing.
fn prepare_crud_mapping(
    params: RequestCrud,
    config: &ConnectionModelDefinition,
) -> Result<RequestCrud, IntegrationOSError> {
    // Remove passthroughForward query param and add user-defined + connection-specific query params
    let (params, removed) = params.remove_query_params("passthroughForward");
    let custom_query_params = removed
        .unwrap_or_default()
        .split('&')
        .filter_map(|pair| {
            pair.split_once('=')
                .map(|(a, b)| (a.to_owned(), b.to_owned()))
        })
        .collect::<HashMap<String, String>>();
    let params = params.extend_query_params(custom_query_params);

    // Remove passthroughHeaders query param and add user-defined + connection-specific headers
    let (params, removed) = params.remove_header("x-integrationos-passthrough-forward");
    let custom_headers: HashMap<HeaderName, HeaderValue> = removed
        .map(|v| v.to_str().map(|s| s.to_string()))
        .map(|s| match s {
            Err(e) => {
                error!(
                    "Failed to convert custom headers to string. ID {:?}, Error: {:?}",
                    config.id, e
                );
                Err(InternalError::invalid_argument(&e.to_string(), None))
            }
            Ok(s) => Ok(s
                .split(';')
                .filter_map(|pair| pair.split_once('='))
                .filter_map(|(a, b)| {
                    match (HeaderName::from_str(a).ok(), HeaderValue::try_from(b).ok()) {
                        (Some(a), Some(b)) => Some((a, b)),
                        _ => None,
                    }
                })
                .collect::<HashMap<HeaderName, HeaderValue>>()),
        })
        .transpose()?
        .unwrap_or_default();

    Ok(params.extend_header(custom_headers))
}
