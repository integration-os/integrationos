use super::ReadResponse;
use crate::server::AppState;
use axum::{
    extract::{Path, State},
    routing::get,
    Json, Router,
};
use bson::{doc, Document};
use futures::StreamExt;
use integrationos_domain::{
    api_model_config::Lang,
    common_model::{CommonEnum, DataType, SchemaType, TypeGenerationStrategy},
    prefix::IdPrefix,
    ApplicationError, Id, IntegrationOSError, InternalError, Store, StringExt,
};
use mongodb::options::FindOptions;
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, sync::Arc};

pub fn get_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/projection", get(get_common_models_projections))
        .route("/:id", get(generate_schema))
        .route("/:id/:type", get(generate_schema))
        .route("/types/:id/:lang", get(generate_types))
        .route("/types/:lang", get(generate_all_types))
        .route("/types/:lang/:models", get(generate_specific_types))
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct GenerateSpecificTypesRequest {
    models: String,
    lang: Lang,
}

async fn generate_specific_types(
    state: State<Arc<AppState>>,
    Path(GenerateSpecificTypesRequest { models, lang }): Path<GenerateSpecificTypesRequest>,
) -> Result<String, IntegrationOSError> {
    let models = models.split(',').map(|s| s.to_string()).collect::<Vec<_>>();
    let cm_store = state.app_stores.common_model.clone();
    let ce_store = state.app_stores.common_enum.clone();

    let visited_enums = &mut HashSet::new();
    let visited_common_models = &mut HashSet::new();

    let common_models = cm_store
        .get_many(
            Some(doc! {
                "name": { "$in": models },
                "deleted": false,
                "active": true,
            }),
            None,
            None,
            None,
            None,
        )
        .await
        .map_err(IntegrationOSError::from)?;

    let mut output_types = String::new();

    for common_model in common_models {
        let expanded = common_model
            .generate_as_expanded(
                &lang,
                &cm_store,
                &ce_store,
                TypeGenerationStrategy::Cumulative {
                    visited_enums,
                    visited_common_models,
                },
            )
            .await;

        output_types.push_str(&expanded);
    }

    Ok(output_types)
}

async fn generate_all_types(
    state: State<Arc<AppState>>,
    Path(lang): Path<Lang>,
) -> Result<String, IntegrationOSError> {
    let cm_store = state.app_stores.common_model.clone();
    let ce_store = state.app_stores.common_enum.clone();

    let mut enums = HashSet::new();

    let common_models = cm_store
        .get_many(
            Some(doc! {
                "deleted": false,
                "active": true,
            }),
            None,
            None,
            None,
            None,
        )
        .await
        .map_err(IntegrationOSError::from)?;

    let common_enums = ce_store
        .get_many(
            Some(doc! {
                "deleted": false,
            }),
            None,
            None,
            None,
            None,
        )
        .await
        .map_err(IntegrationOSError::from)?;

    let mut output_types = String::new();

    for cm in common_models {
        enums.extend(
            cm.get_enum_fields()
                .into_iter()
                .filter_map(|e| match e.datatype {
                    DataType::Enum { options, .. } => Some(CommonEnum {
                        id: Id::now(IdPrefix::CommonEnum),
                        name: e.name.pascal_case(),
                        options: options.unwrap_or_default(),
                    }),
                    _ => None,
                }),
        );

        if cm.name.as_str() == "Collections" {
            continue;
        }

        let Some(lang) = cm.interface.get(&lang) else {
            continue;
        };

        output_types.push_str(lang);
    }

    for ce in enums.iter().chain(common_enums.iter()) {
        match lang {
            Lang::TypeScript => {
                let ts = ce.as_typescript_type();
                output_types.push_str(&ts);
            }
            Lang::Rust => {
                let rust = ce.as_rust_type();
                output_types.push_str(&rust);
            }
            Lang::JavaScript => {
                unimplemented!();
            }
        }
    }

    Ok(output_types)
}

pub async fn get_common_models_projections(
    state: State<Arc<AppState>>,
) -> Result<Json<ReadResponse<Document>>, IntegrationOSError> {
    let collection = state
        .app_stores
        .db
        .collection::<Document>(&Store::CommonModels.to_string());

    let filter = doc! {
        "deleted": false,
        "primary": true,
        "active": true,
    };
    let options = FindOptions::builder()
        .projection(doc! { "_id": 1, "name": 1 })
        .build();

    let mut cursor = collection.find(filter, options).await?;
    let mut common_models: Vec<Document> = Vec::new();

    while let Some(result) = cursor.next().await {
        match result {
            Ok(document) => {
                common_models.push(document);
            }
            _ => {
                return Err(InternalError::unknown(
                    "Error while fetching common models",
                    None,
                ));
            }
        }
    }

    let len = common_models.len();

    Ok(Json(ReadResponse {
        rows: common_models,
        total: len as u64,
        skip: 0,
        limit: 0,
    }))
}

#[derive(Debug, Deserialize)]
struct TypeParams {
    id: Id,
    lang: Lang,
}

async fn generate_types(
    state: State<Arc<AppState>>,
    Path(TypeParams { id, lang }): Path<TypeParams>,
) -> Result<String, IntegrationOSError> {
    let cm_store = state.app_stores.common_model.clone();
    let ce_store = state.app_stores.common_enum.clone();

    let common_model = cm_store
        .get_one_by_id(&id.to_string())
        .await
        .map_err(IntegrationOSError::from)?
        .ok_or(ApplicationError::not_found(
            &format!("CommonModel with id {} not found", id),
            None,
        ))?;

    let schema = common_model
        .generate_as_expanded(&lang, &cm_store, &ce_store, TypeGenerationStrategy::Unique)
        .await;

    Ok(schema)
}

#[derive(Debug, Deserialize)]
pub struct SchemaParams {
    id: Id,
    #[serde(rename = "type")]
    r#type: Option<SchemaType>,
}

pub async fn generate_schema(
    state: State<Arc<AppState>>,
    Path(SchemaParams { id, r#type }): Path<SchemaParams>,
) -> Result<String, IntegrationOSError> {
    let cm_store = state.app_stores.common_model.clone();
    let ce_store = state.app_stores.common_enum.clone();

    let common_model = cm_store
        .get_one_by_id(&id.to_string())
        .await
        .map_err(IntegrationOSError::from)?
        .ok_or(ApplicationError::not_found(
            &format!("CommonModel with id {} not found", id),
            None,
        ))?;

    let schema = common_model
        .as_typescript_schema_expanded(&cm_store, &ce_store, r#type.unwrap_or(SchemaType::Lax))
        .await;

    Ok(schema)
}
