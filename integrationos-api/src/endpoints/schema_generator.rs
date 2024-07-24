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
    api_model_config::Lang, ApplicationError, Id, IntegrationOSError, InternalError, Store,
};
use mongodb::options::FindOptions;
use serde::Deserialize;
use std::sync::Arc;

pub fn get_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/projection", get(get_common_models_projections))
        .route("/:id", get(generate_schema))
        .route("/types/:id/:lang", get(generate_types))
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

pub async fn generate_types(
    state: State<Arc<AppState>>,
    Path(TypeParams { id, lang }): Path<TypeParams>,
) -> Result<String, IntegrationOSError> {
    println!("id: {}, lang: {}", id, lang);

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
        .generate_as_expanded(&lang, &cm_store, &ce_store)
        .await;

    Ok(schema)
}

pub async fn generate_schema(
    state: State<Arc<AppState>>,
    Path(id): Path<Id>,
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
        .as_typescript_schema_expanded(&cm_store, &ce_store)
        .await;

    Ok(schema)
}
