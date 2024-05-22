use super::caller_client::CallerClient;
use crate::{
    api_model_config::{ModelPaths, RequestModelPaths, ResponseModelPaths},
    connection_model_definition::{
        ConnectionModelDefinition, CrudAction, CrudMapping, PlatformInfo,
    },
    connection_model_schema::ConnectionModelSchema,
    database::DatabaseConfig,
    destination::{Action, Destination},
    error::InternalError,
    get_secret_request::GetSecretRequest,
    hashed_secret::HashedSecret,
    id::{prefix::IdPrefix, Id},
    prelude::{CryptoExt, MongoStore, TimedExt},
    Connection, ErrorMeta, IntegrationOSError, Store,
};
use bson::doc;
use chrono::Utc;
use futures::{future::join_all, join, FutureExt};
use handlebars::Handlebars;
use http::{HeaderMap, HeaderName, HeaderValue, Response, StatusCode};
use js_sandbox_ios::Script;
use moka::future::Cache;
use mongodb::{
    options::{Collation, CollationStrength, FindOneOptions},
    Client,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Number, Value};
use std::{
    cell::RefCell,
    collections::HashMap,
    str::FromStr,
    sync::{Arc, RwLock},
};
use tracing::{debug, error};

std::thread_local! {
    static JS_RUNTIME: RefCell<Script> = RefCell::new(Script::new());
}

#[derive(Clone)]
pub struct UnifiedDestination {
    connections_cache: Cache<Arc<str>, Arc<Connection>>,
    connections_store: MongoStore<Connection>,
    connection_model_definitions_cache: Cache<Destination, Arc<ConnectionModelDefinition>>,
    connection_model_definitions_destination_cache:
        Cache<Destination, Arc<ConnectionModelDefinition>>,
    connection_model_definitions_store: MongoStore<ConnectionModelDefinition>,
    connection_model_schemas_cache: Cache<(Arc<str>, Arc<str>), Arc<ConnectionModelSchema>>,
    connection_model_schemas_store: MongoStore<ConnectionModelSchema>,
    secrets_client: Arc<dyn CryptoExt + Sync + Send>,
    secrets_cache: Cache<Connection, Arc<Value>>,
    http_client: reqwest::Client,
    renderer: Option<Arc<RwLock<Handlebars<'static>>>>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestCrudBorrowed<'a> {
    pub query_params: &'a HashMap<String, String>,
    #[serde(with = "http_serde_ext::header_map", default)]
    pub headers: &'a HeaderMap,
    pub path_params: Option<PathParams<'a>>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PathParams<'a> {
    pub id: &'a str,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestCrud {
    pub query_params: Option<HashMap<String, String>>,
    #[serde(with = "http_serde_ext::header_map", default)]
    pub headers: HeaderMap,
    pub path_params: Option<HashMap<String, String>>,
    pub body: Option<Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResponseCrudToMap<'a> {
    #[serde(with = "http_serde_ext::header_map")]
    pub headers: &'a HeaderMap,
    pub pagination: Option<Value>,
    pub request: ResponseCrudToMapRequest<'a>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResponseCrudToMapRequest<'a> {
    pub query_params: &'a HashMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResponseCrud {
    pub pagination: Option<Value>,
}

fn match_route<'a>(full_path: &'a str, routes: impl Iterator<Item = &'a str>) -> Option<&'a str> {
    let path = full_path.split('?').next().unwrap_or("");

    let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();

    for route in routes {
        let route_segments: Vec<&str> = route.split('/').filter(|s| !s.is_empty()).collect();

        if segments.len() != route_segments.len() {
            continue;
        }

        if route_segments
            .iter()
            .zip(&segments)
            .all(|(route_seg, path_seg)| {
                route_seg == path_seg
                    || route_seg.starts_with(':')
                    || (route_seg.starts_with("{{") && route_seg.ends_with("}}"))
            })
        {
            return Some(route);
        }
    }

    None
}

fn template_route(model_definition_path: String, full_request_path: String) -> String {
    let model_definition_segments: Vec<&str> = model_definition_path
        .split('/')
        .filter(|s| !s.is_empty())
        .collect();

    let full_request_segments: Vec<&str> = full_request_path
        .split('/')
        .filter(|s| !s.is_empty())
        .collect();

    let mut template = String::new();

    for (i, segment) in model_definition_segments.iter().enumerate() {
        if segment.starts_with(':') || (segment.starts_with("{{") && segment.ends_with("}}")) {
            template.push_str(full_request_segments[i]);
        } else {
            template.push_str(segment);
        }

        if i != model_definition_segments.len() - 1 {
            template.push('/');
        }
    }

    template
}

fn remove_nulls(value: &mut Value) {
    match value {
        Value::Object(map) => {
            let keys_to_remove: Vec<String> = map
                .iter()
                .filter(|(_, v)| v.is_null())
                .map(|(k, _)| k.clone())
                .collect();

            for key in keys_to_remove {
                map.remove(&key);
            }

            for value in map.values_mut() {
                remove_nulls(value);
            }
        }
        Value::Array(vec) => {
            for item in vec {
                remove_nulls(item);
            }
        }
        _ => {}
    }
}

impl UnifiedDestination {
    pub async fn new(
        config: DatabaseConfig,
        cache_size: u64,
        secrets_client: Arc<dyn CryptoExt + Sync + Send>,
    ) -> Result<Self, IntegrationOSError> {
        let http_client = reqwest::Client::new();
        let connections_cache = Cache::new(cache_size);
        let connection_model_definitions_cache = Cache::new(cache_size);
        let connection_model_definitions_destination_cache = Cache::new(cache_size);
        let connection_model_schemas_cache = Cache::new(cache_size);
        let secrets_cache = Cache::new(cache_size);

        let client = Client::with_uri_str(&config.control_db_url)
            .await
            .map_err(|e| InternalError::connection_error(&e.to_string(), None))?;

        let db = client.database(&config.control_db_name);

        let connections_store = MongoStore::new(&db, &Store::Connections).await?;
        let connection_model_definitions_store =
            MongoStore::new(&db, &Store::ConnectionModelDefinitions).await?;
        let connection_model_schemas_store =
            MongoStore::new(&db, &Store::ConnectionModelSchemas).await?;

        Ok(Self {
            connections_cache,
            connections_store,
            connection_model_definitions_cache,
            connection_model_definitions_destination_cache,
            connection_model_definitions_store,
            connection_model_schemas_cache,
            connection_model_schemas_store,
            secrets_client,
            secrets_cache,
            http_client,
            renderer: if cache_size == 0 {
                None
            } else {
                Some(Arc::new(RwLock::new(Handlebars::new())))
            },
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
                            "action": method.as_str()
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

                let matched_route = match_route(path, routes).map(|r| r.to_string());

                let mut connection_model_definitions = connection_model_definitions
                    .into_iter()
                    .filter(|c| match c.platform_info {
                        PlatformInfo::Api(ref c) => matched_route
                            .as_ref()
                            .map_or(false, |mr| c.path.as_str() == mr),
                    });

                if let Some(connection_model_definition) = connection_model_definitions.next() {
                    if connection_model_definitions.next().is_some() {
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
                .find_one(
                    doc! {
                        "connectionPlatform": destination.platform.as_ref(),
                        "mapping.commonModelName": name.as_ref(),
                        "actionName": action.to_string()
                    },
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
        config: &Arc<ConnectionModelDefinition>,
        headers: HeaderMap,
        query_params: &HashMap<String, String>,
        secret: &Arc<Value>,
        context: Option<Vec<u8>>,
    ) -> Result<reqwest::Response, IntegrationOSError> {
        let template_name = config.id.to_string();
        let config = if let Some(renderer) = &self.renderer {
            let has_template = {
                let guard = renderer.read().unwrap();
                guard.has_template(&template_name)
            };

            if !has_template {
                renderer
                    .write()
                    .unwrap()
                    .register_template_string(
                        &template_name,
                        &serde_json::to_string(&config)
                            .map_err(|e| InternalError::invalid_argument(&e.to_string(), None))?,
                    )
                    .map_err(|e| InternalError::invalid_argument(&e.to_string(), None))?;
            }
            renderer
                .read()
                .unwrap()
                .render(&template_name, secret)
                .map_err(|e| InternalError::invalid_argument(&e.to_string(), None))?
        } else {
            let renderer = Handlebars::new();
            let config = serde_json::to_string(&config)
                .map_err(|e| InternalError::invalid_argument(&e.to_string(), None))?;
            renderer
                .render_template(&config, secret)
                .map_err(|e| InternalError::invalid_argument(&e.to_string(), None))?
        };

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

    pub async fn send_to_destination_unified(
        &self,
        connection: Arc<Connection>,
        action: Action,
        include_passthrough: bool,
        mut headers: HeaderMap,
        mut query_params: HashMap<String, String>,
        mut body: Option<Value>,
    ) -> Result<Response<Value>, IntegrationOSError> {
        let key = Destination {
            platform: connection.platform.clone(),
            action: action.clone(),
            connection_key: connection.key.clone(),
        };

        let config_fut = self
            .connection_model_definitions_cache
            .try_get_with_by_ref(&key, async {
                match self.get_connection_model_definition(&key).await {
                    Ok(Some(c)) => Ok(Arc::new(c)),
                    Ok(None) => Err(InternalError::key_not_found("model definition", None)),
                    Err(e) => Err(InternalError::connection_error(e.message().as_ref(), None)),
                }
            });

        let secret_fut = self.secrets_cache.try_get_with_by_ref(&connection, async {
            let secret_request = GetSecretRequest {
                buildable_id: connection.ownership.id.to_string(),
                id: connection.secrets_service_id.clone(),
            };
            match self
                .secrets_client
                .decrypt(&secret_request)
                .map(|v| Some(v).transpose())
                .await
            {
                Ok(Some(c)) => Ok(Arc::new(c)),
                Ok(None) => Err(InternalError::key_not_found("secret", None)),
                Err(e) => Err(InternalError::connection_error(e.message().as_ref(), None)),
            }
        });

        let Action::Unified {
            action: _,
            id,
            name,
        } = &action
        else {
            return Err(InternalError::invalid_argument(
                "Sent a passthrough to the unified send to destination",
                None,
            ));
        };

        let schema_key = (connection.platform.clone(), name.clone());
        let schema_fut =
            self.connection_model_schemas_cache
                .try_get_with_by_ref(&schema_key, async {
                    match self
                        .connection_model_schemas_store
                        .collection
                        .find_one(
                            doc! {
                                "connectionPlatform": connection.platform.as_ref(),
                                "mapping.commonModelName": name.as_ref(),
                            },
                            FindOneOptions::builder()
                                .collation(Some(
                                    Collation::builder()
                                        .strength(CollationStrength::Secondary)
                                        .locale("en")
                                        .build(),
                                ))
                                .build(),
                        )
                        .await
                    {
                        Ok(Some(c)) => Ok(Arc::new(c)),
                        Ok(None) => Err(InternalError::key_not_found("model schema", None)),
                        Err(e) => Err(InternalError::connection_error(&e.to_string(), None)),
                    }
                });

        let join_result = join!(config_fut, secret_fut, schema_fut);

        let config = join_result.0.map_err(|e| {
            error!("Could not find connection model definition for destination: {e}");
            InternalError::key_not_found("model definition", None)
        })?;

        let mut secret = join_result
            .1
            .map_err(|e| InternalError::key_not_found(&format!("secret for key: {e}"), None))?;

        let cms = join_result.2.map_err(|e| {
            InternalError::key_not_found(&format!("model schema {name} for destination: {e}"), None)
        })?;
        let ConnectionModelSchema {
            id: schema_id,
            mapping,
            ..
        } = cms.as_ref();

        if let Some(id) = id {
            let secret = Arc::make_mut(&mut secret);
            if let Value::Object(sec) = secret {
                const ID: &str = "id";
                sec.insert(ID.to_string(), Value::String(id.to_string()));
            }
        }

        let crud_script_namespace = if self.secrets_cache.policy().max_capacity() == Some(0) {
            "$".to_string() + &uuid::Uuid::new_v4().simple().to_string()
        } else {
            config.id.to_string().replace([':', '-'], "_")
        };
        let schema_script_namespace = if self.secrets_cache.policy().max_capacity() == Some(0) {
            "$".to_string() + &uuid::Uuid::new_v4().simple().to_string()
        } else {
            schema_id.to_string().replace([':', '-'], "_")
        };

        body = if let Some(body) = body {
            if let Some(js) = mapping.as_ref().map(|m| m.from_common_model.as_str()) {
                debug!(
                    "Mapping request body {}\nUsing js {js}",
                    serde_json::to_string_pretty(&body).map_err(|e| {
                        error!("Could not convert body to pretty string {body:?}: {e}");
                        InternalError::invalid_argument(&e.to_string(), None)
                    })?
                );

                let ns: String = schema_script_namespace.clone() + "_mapFromCommonModel";
                JS_RUNTIME
                    .with_borrow_mut(|script| script.add_script(&ns, "mapFromCommonModel", js))
                    .map_err(|e| {
                        error!("Could not create request schema mapping script: {e}");
                        InternalError::invalid_argument(&e.to_string(), None)
                    })?;
                let mut body = JS_RUNTIME
                    .with_borrow_mut(|script| script.call_namespace(&ns, body))
                    .map_err(|e| {
                        InternalError::script_error(
                            &format!("Failed while running request schema mapping script: {e}"),
                            None,
                        )
                    })?;

                tokio::task::yield_now().await;

                remove_nulls(&mut body);

                debug!(
                    "Mapped body to {}",
                    serde_json::to_string_pretty(&body).map_err(|e| {
                        error!("Could not convert body to pretty string {body:?}: {e}");
                        InternalError::invalid_argument(&e.to_string(), None)
                    })?
                );

                Some(body)
            } else {
                debug!(
                    "No js for schema mapping to common model {name} for {}",
                    connection.platform
                );
                Some(body)
            }
        } else {
            debug!("No body to map");
            None
        };

        if let Some(CrudMapping {
            from_common_model: Some(js),
            ..
        }) = &config.mapping
        {
            if !js.is_empty() {
                let ns: String = crud_script_namespace.clone() + "_mapFromCrudRequest";
                JS_RUNTIME
                    .with_borrow_mut(|script| script.add_script(&ns, "mapCrudRequest", js.as_str()))
                    .map_err(|e| {
                        error!("Could not create request crud mapping script from {js}: {e}");
                        InternalError::script_error(&e.to_string(), None)
                    })?;

                const PASSTHROUGH_PARAMS: &str = "passthroughForward";
                const PASSHTROUGH_HEADERS: &str = "x-integrationos-passthrough-forward";

                if let Some(custom_params) = query_params.remove(PASSTHROUGH_PARAMS) {
                    let pairs = custom_params.split('&').filter_map(|pair| {
                        pair.split_once('=')
                            .map(|(a, b)| (a.to_owned(), b.to_owned()))
                    });
                    query_params.extend(pairs);
                }

                if let Some(custom_headers) = headers.remove(PASSHTROUGH_HEADERS) {
                    let pairs = custom_headers
                        .to_str()
                        .map_err(|e| {
                            error!("Bad custom headers value: {e}");
                            InternalError::invalid_argument(&e.to_string(), None)
                        })?
                        .split(';')
                        .filter_map(|pair| pair.split_once('='))
                        .filter_map(|(a, b)| {
                            match (HeaderName::from_str(a).ok(), HeaderValue::try_from(b).ok()) {
                                (Some(a), Some(b)) => Some((Some(a), b)),
                                _ => None,
                            }
                        });
                    headers.extend(pairs);
                }

                let request = RequestCrudBorrowed {
                    query_params: &query_params,
                    headers: &headers,
                    path_params: id.as_ref().map(|id| PathParams { id }),
                };

                debug!(
                    "Mapping request crud {}\nUsing js {js}",
                    serde_json::to_string_pretty(&request).map_err(|e| {
                        error!("Could not convert request crud to pretty string {request:?}: {e}");
                        InternalError::invalid_argument(&e.to_string(), None)
                    })?
                );

                let res: RequestCrud = JS_RUNTIME
                    .with_borrow_mut(|script| script.call_namespace(&ns, request))
                    .map_err(|e| {
                        InternalError::script_error(
                            &format!("Failed while running request crud mapping script: {e}"),
                            None,
                        )
                    })?;

                debug!(
                    "Mapped request crud to {}",
                    serde_json::to_string_pretty(&res).map_err(|e| {
                        error!("Could not convert crud to pretty string {res:?}: {e}");
                        InternalError::invalid_argument(&e.to_string(), None)
                    })?
                );

                headers = res.headers;

                query_params = res.query_params.unwrap_or_default();

                let secret = Arc::make_mut(&mut secret);
                if let Value::Object(ref mut sec) = secret {
                    if let Some(path_params) = res.path_params {
                        sec.extend(path_params.into_iter().map(|(a, b)| (a, Value::String(b))));
                    }
                }

                match (&mut body, res.body) {
                    (Some(Value::Object(a)), Some(Value::Object(b))) => {
                        a.extend(b);
                    }
                    (body @ None, Some(mapped_body)) => {
                        body.replace(mapped_body);
                    }
                    _ => {}
                }
            }
        }

        let PlatformInfo::Api(api_config) = &config.platform_info;

        if let Some(ModelPaths {
            request: Some(RequestModelPaths { object: Some(path) }),
            ..
        }) = &api_config.paths
        {
            if let Some(path) = path.strip_prefix("$.body.") {
                body = body.map(|body| json!({path: body}));
                debug!(
                    "Mapped request body to {path}: {}",
                    serde_json::to_string_pretty(&body).map_err(|e| {
                        error!("Could not convert mapped body to pretty string {body:?}: {e}");
                        InternalError::invalid_argument(&e.to_string(), None)
                    })?
                );
            }
        }

        debug!("Executing model definition with config {config:#?}, headers {headers:#?}, query params {query_params:#?}");

        let context = match body {
            None | Some(Value::Null) => None,
            _ => Some(serde_json::to_vec(&body).map_err(|e| {
                error!("Could not convert body to vec: {e}");
                InternalError::invalid_argument(&e.to_string(), None)
            })?),
        };

        let mut latency = 0i64;
        let mut res = self
            .execute_model_definition(&config, headers, &query_params, &secret, context)
            .timed(|_, duration| {
                latency = duration.as_millis() as i64;
            })
            .await?;

        debug!(
            "Executed model definition with status code {}, headers: {:#?}",
            res.status(),
            res.headers()
        );

        let headers = std::mem::take(res.headers_mut());

        if !res.status().is_success() {
            let status = res.status();

            let mut res = Response::builder()
                .status(status)
                .body(res.json().await.map_err(|e| {
                    error!("Could not get json body from unsuccessful response");
                    IntegrationOSError::from_err_code(status, &e.to_string(), None)
                })?)
                .map_err(|e| {
                    error!("Could not create response from builder for unsucessful response");
                    IntegrationOSError::from_err_code(status, &e.to_string(), None)
                })?;
            *res.headers_mut() = headers;
            return Ok(res);
        }

        let status = res.status();

        let mut body: Option<Value> = res.json().await.ok();

        let passthrough = if include_passthrough {
            body.clone()
        } else {
            None
        };

        debug!(
            "Received response body: {}",
            serde_json::to_string_pretty(&body).map_err(|e| {
                error!("Could not convert mapped body to pretty string {body:?}: {e}");
                InternalError::invalid_argument(&e.to_string(), None)
            })?
        );

        let pagination = if config.action_name == CrudAction::GetMany {
            if let Some(CrudMapping {
                to_common_model: Some(js),
                ..
            }) = &config.mapping
            {
                if !js.is_empty() {
                    let ns: String = crud_script_namespace + "_mapToCrudRequest";
                    JS_RUNTIME
                        .with_borrow_mut(|script| {
                            script.add_script(&ns, "mapCrudRequest", js.as_str())
                        })
                        .map_err(|e| {
                            error!("Could not create response crud mapping script from {js}: {e}");
                            InternalError::script_error(&e.to_string(), None)
                        })?;

                    let pagination = if let (
                        Some(ModelPaths {
                            response:
                                Some(ResponseModelPaths {
                                    cursor: Some(path), ..
                                }),
                            ..
                        }),
                        Some(body),
                    ) = (&api_config.paths, &body)
                    {
                        let wrapped_body = json!({"body":body});
                        let mut bodies =
                            jsonpath_lib::select(&wrapped_body, path).map_err(|e| {
                                error!("Could not select cursor at response path {path}: {e}");
                                InternalError::invalid_argument(&e.to_string(), None)
                            })?;
                        if bodies.len() != 1 {
                            Some(Value::Null)
                        } else {
                            Some(bodies.remove(0).clone())
                        }
                    } else {
                        None
                    };

                    let res_to_map = ResponseCrudToMap {
                        headers: &headers,
                        pagination,
                        request: ResponseCrudToMapRequest {
                            query_params: &query_params,
                        },
                    };

                    debug!(
                        "Mapping response crud {}\nUsing js {js}",
                        serde_json::to_string_pretty(&res_to_map).map_err(|e| {
                            error!("Could not convert response crud to pretty string {res_to_map:?}: {e}");
                            InternalError::invalid_argument(&e.to_string(), None)
                        })?
                    );

                    let res: ResponseCrud = JS_RUNTIME
                        .with_borrow_mut(|script| script.call_namespace(&ns, &res_to_map))
                        .map_err(|e| {
                            InternalError::script_error(
                                &format!("Failed while running response crud mapping script: {e}"),
                                None,
                            )
                        })?;

                    tokio::task::yield_now().await;

                    debug!(
                        "Mapped response crud to {}",
                        serde_json::to_string_pretty(&res).map_err(|e| {
                            error!("Could not convert crud to pretty string {res:?}: {e}");
                            InternalError::invalid_argument(&e.to_string(), None)
                        })?
                    );

                    res.pagination
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };

        if let Some(ModelPaths {
            response:
                Some(ResponseModelPaths {
                    object: Some(path), ..
                }),
            ..
        }) = &api_config.paths
        {
            body = if let Some(body) = body {
                let wrapped_body = json!({"body":body});
                let mut bodies = jsonpath_lib::select(&wrapped_body, path).map_err(|e| {
                    error!("Could not select body at response path {path}: {e}");
                    InternalError::invalid_argument(&e.to_string(), None)
                })?;
                if bodies.is_empty() {
                    let error_string = format!(
                        "Could not map unified model. 3rd party Connection returned an invalid response. Expected model at path {path} but found none.",
                    );
                    let mut res = Response::builder()
                        .status(StatusCode::UNPROCESSABLE_ENTITY)
                        .body(json!({
                            "message": error_string,
                            "passthrough": wrapped_body
                        }))
                        .map_err(|e| {
                            error!("Could not create response from builder for missing body");
                            IntegrationOSError::from_err_code(
                                StatusCode::UNPROCESSABLE_ENTITY,
                                &e.to_string(),
                                None,
                            )
                        })?;
                    *res.headers_mut() = headers;
                    return Ok(res);
                }
                if bodies.len() != 1 {
                    return Err(InternalError::invalid_argument(
                        &format!(
                            "Invalid number of selected bodies ({}) at response path {path}",
                            bodies.len()
                        ),
                        None,
                    ));
                }
                Some(bodies.remove(0).clone())
            } else {
                None
            };
            debug!(
                "Mapped response body to {path}: {}",
                serde_json::to_string_pretty(&body).map_err(|e| {
                    error!("Could not convert mapped body to pretty string {body:?}: {e}");
                    InternalError::invalid_argument(&e.to_string(), None)
                })?
            );
        }

        if matches!(
            config.action_name,
            CrudAction::GetMany | CrudAction::GetOne | CrudAction::Create
        ) {
            let Some(js) = mapping.as_ref().map(|m| &m.to_common_model) else {
                return Err(InternalError::invalid_argument(
                    &format!(
                        "No js for schema mapping to common model {name} for {}",
                        connection.platform
                    ),
                    None,
                ));
            };
            let ns: String = schema_script_namespace + "_mapToCommonModel";
            JS_RUNTIME
                .with_borrow_mut(|script| script.add_script(&ns, "mapToCommonModel", js))
                .map_err(|e| {
                    error!("Could not create response schema mapping script from {js}: {e}");
                    InternalError::script_error(&e.to_string(), None)
                })?;

            debug!(
                "Mapping response body {}\nUsing js {js}",
                serde_json::to_string_pretty(&body).map_err(|e| {
                    error!("Could not convert body to pretty string {body:?}: {e}");
                    InternalError::invalid_argument(&e.to_string(), None)
                })?
            );

            const ID_KEY: &str = "id";
            const MODIFY_TOKEN_KEY: &str = "modifyToken";

            let mut mapped_body: Value = if let Some(Value::Array(arr)) = body {
                let mut futs = Vec::with_capacity(arr.len());
                for body in arr {
                    futs.push(async {
                        let res =
                            JS_RUNTIME.with_borrow_mut(|script| {
                                script
                            .add_script(&ns, "mapToCommonModel", js)
                            .and_then(|_| script.call_namespace(&ns, body))
                            .map_err(|e| {
                                InternalError::script_error(
                                    &format!("Failed while running response schema mapping script: {e}"),
                                    None,
                                )
                            })
                            });
                        tokio::task::yield_now().await;
                        res.map(|mut body| {
                            if let Value::Object(map) = &mut body {
                                if !map.contains_key(MODIFY_TOKEN_KEY) {
                                    let v = map.get(ID_KEY).cloned().unwrap_or(json!(""));
                                    map.insert(MODIFY_TOKEN_KEY.to_owned(), v);
                                }
                            }
                            body
                        })
                    });
                }
                let values = join_all(futs)
                    .await
                    .into_iter()
                    .collect::<Result<Vec<Value>, _>>()?;
                Value::Array(values)
            } else if let Some(body) = &body {
                JS_RUNTIME
                    .with_borrow_mut(|script| script.call_namespace(&ns, body))
                    .map(|mut body| {
                        if let Value::Object(map) = &mut body {
                            if !map.contains_key(MODIFY_TOKEN_KEY) {
                                let v = map.get(ID_KEY).cloned().unwrap_or(json!(""));
                                map.insert(MODIFY_TOKEN_KEY.to_owned(), v);
                            }
                        }
                        body
                    })
                    .map_err(|e| {
                        InternalError::script_error(
                            &format!("Failed while running response schema mapping script: {e}"),
                            None,
                        )
                    })?
            } else if matches!(config.action_name, CrudAction::GetMany) {
                Value::Array(Default::default())
            } else {
                Value::Object(Default::default())
            };

            remove_nulls(&mut mapped_body);

            body = Some(mapped_body);
        } else if matches!(config.action_name, CrudAction::Update | CrudAction::Delete) {
            body = None;
        }

        debug!(
            "Mapped response body to {}",
            serde_json::to_string_pretty(&body).map_err(|e| {
                error!("Could not convert body to pretty string {body:?}: {e}");
                InternalError::invalid_argument(&e.to_string(), None)
            })?
        );

        let mut response = json!({});

        let response_len = if let Some(Value::Array(arr)) = &body {
            arr.len()
        } else {
            0
        };

        let hash = HashedSecret::try_from(json!({
            "response": &body,
            "action": config.action_name,
            "commonModel": config.mapping.as_ref().map(|m| &m.common_model_name),
        }))?;

        if let (Some(body), Value::Object(ref mut response)) = (body, &mut response) {
            const UNIFIED: &str = "unified";
            if config.action_name == CrudAction::GetCount {
                const COUNT: &str = "count";
                response.insert(UNIFIED.to_string(), json!({ COUNT: body }));
            } else {
                response.insert(UNIFIED.to_string(), body);
            }
        }

        if let (true, Some(passthrough), Value::Object(ref mut response)) =
            (include_passthrough, passthrough, &mut response)
        {
            const PASSTHROUGH: &str = "passthrough";
            response.insert(PASSTHROUGH.to_string(), passthrough);
        }

        if let (Some(Value::Object(mut pagination)), Value::Object(ref mut response)) =
            (pagination, &mut response)
        {
            const LIMIT: &str = "limit";
            if let Some(Ok(limit)) = query_params.get(LIMIT).map(|s| s.parse::<u32>()) {
                pagination.insert(LIMIT.to_string(), Value::Number(Number::from(limit)));
            }
            const PAGE_SIZE: &str = "pageSize";
            pagination.insert(
                PAGE_SIZE.to_string(),
                Value::Number(Number::from(response_len)),
            );
            const PAGINATION: &str = "pagination";
            response.insert(PAGINATION.to_string(), Value::Object(pagination));
        }

        if let Value::Object(ref mut response) = &mut response {
            let meta = json!({
                "timestamp": Utc::now().timestamp_millis(),
                "latency": latency,
                "platformRateLimitRemaining": 0,
                "rateLimitRemaining": 0,
                "cache": {
                    "hit": false,
                    "ttl": 0,
                    "key": ""
                },
                "transactionKey": Id::now(IdPrefix::Transaction),
                "platform": connection.platform,
                "platformVersion": connection.platform_version,
                "action": config.action_name,
                "commonModel": config.mapping.as_ref().map(|m| &m.common_model_name),
                "commonModelVersion": "v1",
                "connectionKey": connection.key,
                "hash": hash.inner(),
            });
            const META: &str = "meta";
            response.insert(META.to_string(), meta);
        }

        let mut builder = Response::builder();

        if status.is_success() {
            const STATUS_HEADER: &str = "response-status";
            builder = builder
                .header::<&'static str, HeaderValue>(STATUS_HEADER, status.as_u16().into())
                .status(StatusCode::OK);
        } else {
            builder = builder.status(status);
        }
        if let Some(builder_headers) = builder.headers_mut() {
            builder_headers.extend(headers.into_iter());
        } else {
            return Err(IntegrationOSError::from_err_code(
                status,
                "Could not get headers from builder",
                None,
            ));
        };
        let res = builder.body(response).map_err(|e| {
            error!("Could not create response from builder");
            IntegrationOSError::from_err_code(status, &e.to_string(), None)
        })?;

        Ok(res)
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
            self.connections_cache
                .try_get_with_by_ref(&destination.connection_key, async {
                    match self
                        .connections_store
                        .get_one(doc! { "key": destination.connection_key.as_ref() })
                        .await
                    {
                        Ok(Some(c)) => Ok(Arc::new(c)),
                        Ok(None) => Err(InternalError::key_not_found("Connection", None)),
                        Err(e) => Err(InternalError::connection_error(e.message().as_ref(), None)),
                    }
                })
                .await
                .map_err(|e| {
                    InternalError::connection_error(
                        &e.to_string(),
                        Some(&destination.connection_key.clone()),
                    )
                })?
        };

        let config = match self.get_connection_model_definition(destination).await {
            Ok(Some(c)) => Ok(Arc::new(c)),
            Ok(None) => Err(InternalError::key_not_found(
                "ConnectionModelDefinition",
                None,
            )),
            Err(e) => Err(InternalError::connection_error(e.message().as_ref(), None)),
        }?;

        let secret = self
            .secrets_cache
            .try_get_with_by_ref(&connection, async {
                let secret_request = GetSecretRequest {
                    buildable_id: connection.ownership.id.to_string(),
                    id: connection.secrets_service_id.clone(),
                };
                match self
                    .secrets_client
                    .decrypt(&secret_request)
                    .map(|v| Some(v).transpose())
                    .await
                {
                    Ok(Some(c)) => Ok(Arc::new(c)),
                    Ok(None) => Err(InternalError::key_not_found("Secrets", None)),
                    Err(e) => Err(InternalError::connection_error(e.message().as_ref(), None)),
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

        self.execute_model_definition(&templated_config, headers, &query_params, &secret, context)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_match_route() {
        let routes = [
            "/customers",
            "/customers/:id",
            "/customers/{{id}}/orders",
            "/customers/:id/orders/:order_id",
        ]
        .into_iter();

        assert_eq!(
            match_route("/customers", routes.clone()),
            Some("/customers")
        );
        assert_eq!(
            match_route("/customers/123", routes.clone()),
            Some("/customers/:id")
        );
        assert_eq!(
            match_route("/customers/123/orders", routes.clone()),
            Some("/customers/{{id}}/orders")
        );
        assert_eq!(
            match_route("/customers/123/orders/456", routes.clone()),
            Some("/customers/:id/orders/:order_id")
        );
        assert_eq!(match_route("/customers/123/456", routes.clone()), None);
        assert_eq!(match_route("/customers/123/orders/456/789", routes), None);
    }

    #[test]
    fn test_template_route() {
        assert_eq!(
            template_route(
                "/customers/:id/orders/:order_id".to_string(),
                "/customers/123/orders/456".to_string()
            ),
            "customers/123/orders/456".to_string()
        );

        assert_eq!(
            template_route(
                "/customers/{{id}}/orders/{{order_id}}".to_string(),
                "/customers/123/orders/456".to_string()
            ),
            "customers/123/orders/456".to_string()
        );
    }
}
