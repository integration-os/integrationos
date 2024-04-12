use super::ApiError;
use crate::{
    debug_error, internal_server_error,
    server::AppState,
    util::{generate_openapi_schema, generate_path_item},
};
use axum::extract::{Json, State};
use bson::doc;
use convert_case::{Case, Casing};
use futures::{Stream, StreamExt, TryStreamExt};
use http::StatusCode;
use indexmap::IndexMap;
use integrationos_domain::{
    algebra::{MongoStore, StoreExt, TimedExt},
    common_model::{CommonEnum, CommonModel},
};
use mongodb::error::Error as MongoError;
use openapiv3::*;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    pin::Pin,
    sync::{Arc, RwLock},
};
use tokio::task::JoinHandle;
use tracing::{debug, error, info};

#[derive(Clone, Default, Debug)]
pub struct OpenAPIData {
    state: Arc<RwLock<CachedSchema>>,
}

impl OpenAPIData {
    pub fn get(&self) -> Result<CachedSchema, anyhow::Error> {
        self.state.read().map(|state| state.clone()).map_err(|e| {
            anyhow::Error::msg(format!("Could not get openapi schema from cache: {e}"))
        })
    }

    pub fn set(&self, value: CachedSchema) -> Result<(), anyhow::Error> {
        self.state
            .write()
            .map(|mut state| *state = value)
            .map_err(|e| anyhow::Error::msg(format!("Could not set openapi schema in cache: {e}")))
    }

    pub fn clear(&self) -> Result<(), anyhow::Error> {
        self.set(CachedSchema::default())
    }

    pub fn spawn_openapi_generation(
        &self,
        cm_store: MongoStore<CommonModel>,
        ce_store: MongoStore<CommonEnum>,
    ) -> JoinHandle<Result<(), anyhow::Error>> {
        spawn_openapi_generation(cm_store, ce_store, self.clone())
    }
}

#[derive(Debug, Clone, Default)]
pub struct CachedSchema {
    schema: Vec<u8>,
    is_generating: bool,
    error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", untagged)]
pub enum OpenApiSchema {
    OpenAPI(OpenAPI),
    Accepted(String),
    Error(String),
}

struct PathWithSchema {
    path: IndexMap<String, ReferenceOr<PathItem>>,
    schema: IndexMap<String, ReferenceOr<Schema>>,
}

struct PathIter {
    paths: Vec<IndexMap<String, ReferenceOr<PathItem>>>,
    components: IndexMap<String, ReferenceOr<Schema>>,
}

impl PathIter {
    /// Takes a list of paths and components, merges the components, collects
    /// all the paths and returns a PathIter
    fn from_paths(paths: Vec<PathWithSchema>) -> Self {
        let mut components = IndexMap::new();

        for path in &paths {
            components.extend(path.schema.clone());
        }

        let paths = paths
            .into_iter()
            .map(|path| path.path)
            .collect::<Vec<IndexMap<String, ReferenceOr<PathItem>>>>();

        Self { paths, components }
    }
}

type StreamResult = Pin<Box<dyn Stream<Item = Result<CommonModel, MongoError>> + Send>>;

#[tracing::instrument(name = "Refresh OpenAPI schema", skip(state))]
pub async fn refresh_openapi(
    state: State<Arc<AppState>>,
) -> Result<(StatusCode, Json<OpenApiSchema>), ApiError> {
    state.openapi_data.clone().clear().map_err(|e| {
        error!("Could not clear openapi schema from cache: {:?}", e);
        internal_server_error!()
    })?;

    spawn_openapi_generation(
        state.app_stores.common_model.clone(),
        state.app_stores.common_enum.clone(),
        state.openapi_data.clone(),
    );

    Ok((
        StatusCode::ACCEPTED,
        Json(OpenApiSchema::Accepted(
            "OpenAPI schema is being regenerated".to_string(),
        )),
    ))
}

#[tracing::instrument(name = "Get OpenAPI schema", skip(state))]
pub async fn get_openapi(
    state: State<Arc<AppState>>,
) -> Result<(StatusCode, Json<OpenApiSchema>), ApiError> {
    let schema = state.openapi_data.get().map_err(|e| {
        error!("Could not get openapi schema from cache: {:?}", e);
        internal_server_error!()
    })?;

    if schema.is_generating {
        info!("OpenAPI schema is being generated");
        return Ok((
            StatusCode::ACCEPTED,
            Json(OpenApiSchema::Accepted(
                "You're early, the schema is being generated".to_string(),
            )),
        ));
    }

    if let Some(error) = &schema.error {
        info!("OpenAPI schema generation failed: {}, retrying...", error);
        spawn_openapi_generation(
            state.app_stores.common_model.clone(),
            state.app_stores.common_enum.clone(),
            state.openapi_data.clone(),
        );
        return Err(debug_error!(format!(
            "OpenAPI schema generation failed: {}",
            error
        )));
    }

    let openapi = serde_json::from_slice(schema.schema.as_ref()).map_err(|e| {
        error!("Could not deserialize openapi schema: {:?}", e);
        internal_server_error!()
    })?;

    Ok((StatusCode::OK, Json(OpenApiSchema::OpenAPI(openapi))))
}

fn spawn_openapi_generation(
    cm_store: MongoStore<CommonModel>,
    ce_store: MongoStore<CommonEnum>,
    state: OpenAPIData,
) -> JoinHandle<Result<(), anyhow::Error>> {
    tokio::spawn(async move {
        let stream: StreamResult = cm_store
            .collection
            .find(Some(doc! { "primary": true }), None)
            .await
            .map_err(|e| {
                error!("Could not fetch common model: {:?}", e);
                e
            })?
            .boxed();

        let cached_schema = CachedSchema {
            schema: Vec::new(),
            is_generating: true,
            error: None,
        };

        info!("Setting openapi schema as generating in cache");
        state.set(cached_schema.clone()).map_err(|e| {
            error!("Could not set openapi schema as generating in cache: {e}");
            e
        })?;

        let result = stream
            .map(|cm| async {
                let cm_store = cm_store.clone();
                let ce_store = ce_store.clone();
                match cm {
                    Ok(cm) => Some(
                        generate_references_data(cm, cm_store, ce_store)
                            .timed(|_, elapsed| {
                                debug!("Common model processed in {:?}", elapsed);
                            })
                            .await,
                    ),
                    Err(e) => {
                        error!("Could not fetch common model: {e}");
                        None
                    }
                }
            })
            .buffer_unordered(10)
            .filter_map(|x| async { x })
            .try_collect::<Vec<PathWithSchema>>()
            .await;

        match result {
            Ok(paths) => {
                info!("Generating openapi schema");
                let paths = PathIter::from_paths(paths);
                let schema = generate_openapi_schema(paths.paths, paths.components);

                info!("Deserializing openapi schema");
                let schema = serde_json::to_vec(&schema).map_err(|e| {
                    error!("Could not serialize openapi schema: {e}");
                    e
                });

                if schema.is_err() {
                    state
                        .set(CachedSchema {
                            schema: vec![],
                            is_generating: false,
                            error: Some(
                                "Could not serialize openapi schema, retrying...".to_string(),
                            ),
                        })
                        .map_err(|e| {
                            error!("Could not set openapi schema in cache: {e}");
                            e
                        })?;
                }

                info!("Setting openapi schema in cache");
                if let Ok(schema) = schema {
                    state
                        .set(CachedSchema {
                            schema,
                            is_generating: false,
                            error: None,
                        })
                        .map_err(|e| {
                            error!("Could not set openapi schema in cache: {e}");
                            e
                        })?;
                }
                Ok(())
            }
            Err(err) => {
                error!("Could not generate openapi schema: {err}");
                state
                    .set(CachedSchema {
                        schema: vec![],
                        is_generating: false,
                        error: Some(format!("Could not generate openapi schema: {err}")),
                    })
                    .map_err(|e| {
                        error!("Could not set openapi schema in cache: {e}");
                        e
                    })
            }
        }
    })
}

async fn generate_references_data(
    cm: CommonModel,
    cm_store: MongoStore<CommonModel>,
    ce_store: MongoStore<CommonEnum>,
) -> Result<PathWithSchema, anyhow::Error> {
    let mut schema = IndexMap::new();
    let (child_cms, missing) = cm
        .fetch_all_children_common_models(cm_store.clone())
        .await?;
    // PERF: Use fetch_all_children_common_enums instead
    let mut enum_references = cm
        .get_enum_references()
        .into_iter()
        .filter_map(|x| match x.datatype {
            integrationos_domain::common_model::DataType::Enum { reference, .. } => {
                Some(reference.to_case(Case::Pascal))
            }
            _ => None,
        })
        .collect::<HashSet<_>>();

    if !missing.is_empty() {
        debug!("Missing children. Contact platform to create {:?}", missing);
    }

    // Add properties for children
    for (k, child_cm) in child_cms.into_iter() {
        schema.insert(k, ReferenceOr::Item(child_cm.reference()));
        let references = child_cm
            .get_enum_references()
            .into_iter()
            .filter_map(|x| match x.datatype {
                integrationos_domain::common_model::DataType::Enum { reference, .. } => {
                    Some(reference.to_case(Case::Pascal))
                }
                _ => None,
            })
            .collect::<HashSet<_>>();

        enum_references.extend(references);
    }

    // Add properties for enum references
    let enum_references = ce_store
        .get_many(
            Some(doc! {
                "name": {
                    "$in": bson::to_bson(&enum_references)?
                }
            }),
            None,
            None,
            None,
            None,
        )
        .await?;

    enum_references.into_iter().for_each(|ce| {
        schema.insert(
            ce.name.clone(),
            ReferenceOr::Item(Schema {
                schema_data: Default::default(),
                schema_kind: SchemaKind::Type(Type::String(StringType {
                    format: VariantOrUnknownOrEmpty::Unknown(ce.name.to_case(Case::Camel)),
                    enumeration: ce
                        .options
                        .iter()
                        .map(|option| Some(option.to_owned()))
                        .collect(),
                    ..Default::default()
                })),
            }),
        );
    });

    // Add dummy properties for missing children
    for r#ref in missing {
        let schema_item = Schema {
            schema_data: Default::default(),
            schema_kind: SchemaKind::Type(Type::Object(ObjectType {
                properties: {
                    IndexMap::from_iter(vec![(
                        r#ref.clone(),
                        ReferenceOr::Item(Box::new(Schema {
                            schema_data: Default::default(),
                            schema_kind: SchemaKind::Type(Type::Object(ObjectType {
                                properties: Default::default(),
                                ..Default::default()
                            })),
                        })),
                    )])
                },
                ..Default::default()
            })),
        };
        schema.insert(r#ref.clone(), ReferenceOr::Item(schema_item));
    }

    // Add properties for the common model itself
    schema.insert(cm.name.clone(), ReferenceOr::Item(cm.reference()));

    let path = generate_path_item(&cm);
    Ok(PathWithSchema { path, schema })
}
