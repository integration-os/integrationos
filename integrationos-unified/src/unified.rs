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
    pub metadata: Value,
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

    pub async fn get_dependencies(
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

    pub async fn send_to_destination_unified(
        &self,
        connection: Arc<Connection>,
        action: Action,
        environment: Environment,
        params: RequestCrud,
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
                passthrough,
            } => {
                // (ConnectionModelDefinition, Secret, ConnectionModelSchema)
                let (config, secret, cms) = self.get_dependencies(&key, &connection, &name).await.inspect_err(|e| {
                    error!("Failed to get dependencies for unified destination. Destination: {:?}, Error: {e}", key);
                })?;

                let secret = insert_action_id(secret.as_value()?, id.as_ref());

                // Namespace for js scripts
                let crud_namespace = generate_script_namespace(self.secrets_cache.max_capacity(), &config.id.to_string());
                let schema_namespace = generate_script_namespace(self.secrets_cache.max_capacity(), &cms.id.to_string());

                let metadata = UnifiedMetadataBuilder::default()
                    .timestamp(Utc::now().timestamp_millis())
                    .platform_rate_limit_remaining(0)
                    .rate_limit_remaining(0)
                    .host(params.get_header("host"))
                    .transaction_key(Id::now(IdPrefix::Transaction))
                    .platform(connection.platform.to_string())
                    .platform_version(connection.platform_version.to_string())
                    .action(config.action_name.to_string())
                    .common_model(config.mapping.as_ref().map(|m| m.common_model_name.clone()).unwrap_or_default())
                    .common_model_version("v1")
                    .connection_key(connection.key.to_string());


                let body = params.get_body().ok_or_else(|| InternalError::invalid_argument("No body found", None))?;

                // TODO: Return this body
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

                let crud_mapping: Option<Result<(), IntegrationOSError>> = OptionFuture::from(config.mapping.as_ref().map(|m| m.from_common_model.to_owned())
                    .map(|code| async {
                        match code {
                            None => Ok(()),
                            Some(code) => {
                                let namespace = crud_namespace.clone() + "_mapFromCrudRequest";
                                let jsruntime = JSRuntimeImplBuilder::default().namespace(namespace).code(code).build()
                                    .inspect_err(|e| {
                                        error!("Failed to create request schema mapping script for connection model. ID: {}, Error: {}", config.id, e);
                                    })?;
                                let jsruntime = jsruntime.create("mapCrudRequest")?;

                                let params: RequestCrud = prepare_crud_mapping(params, &config)?;


                                let params: RequestCrud = jsruntime.run(&params).await?;
                                    // JS_RUNTIME
                                    // .with_borrow_mut(|script| script.call_namespace(&ns, request))
                                    // .map_err(|e| {
                                    //     error!("Failed to run request crud mapping script for connection model. ID: {}, Error: {}", config.id, e);
                                    // 
                                    //     ApplicationError::bad_request(
                                    //         &format!("Failed while running request crud mapping script: {e}"),
                                    //         None,
                                    //     )
                                    //         .set_meta(&metadata)
                                    // })?;
                                Ok(())
                            }
                        }
                    })).await;


                todo!()
            }
            Action::Passthrough { method, path } => Err(InternalError::invalid_argument(
                &format!("Passthrough action is not supported for destination {}, in method {method} and path {path}", key.connection_key),
                None,
            )),
        }
    }

    pub async fn send_to_destination(
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
