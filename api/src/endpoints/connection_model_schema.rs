use super::{create, delete, read, update, ApiResult, HookExt, RequestExt};
use crate::{
    internal_server_error,
    server::{AppState, AppStores},
};
use axum::{
    extract::{Path, State},
    routing::{patch, post},
    Json, Router,
};
use integrationos_domain::{
    algebra::{MongoStore, StoreExt},
    connection_model_schema::{ConnectionModelSchema, Mappings, SchemaPaths},
    id::{prefix::IdPrefix, Id},
    json_schema::JsonSchema,
};
use mongodb::bson::doc;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use tracing::error;

pub fn get_router() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/",
            post(create::<CreateRequest, ConnectionModelSchema>)
                .get(read::<CreateRequest, ConnectionModelSchema>),
        )
        .route(
            "/:id",
            patch(update::<CreateRequest, ConnectionModelSchema>)
                .delete(delete::<CreateRequest, ConnectionModelSchema>),
        )
}

pub async fn get_platform_models(
    Path(platform_name): Path<String>,
    State(state): State<Arc<AppState>>,
) -> ApiResult<Vec<String>> {
    let store = state.app_stores.public_model_schema.clone();

    let res = store
        .get_many(
            Some(doc! {
                "connectionPlatform": &platform_name,
                "mapping": { "$ne": null }
            }),
            None,
            None,
            Some(100),
            None,
        )
        .await
        .map_err(|e| {
            error!("Error reading from connection model schema store: {e}");
            internal_server_error!()
        })?;

    let common_model_names = res
        .into_iter()
        .map(|r| r.mapping)
        .map(|m| m.common_model_name)
        .collect::<Vec<String>>();

    Ok(Json(common_model_names))
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[cfg_attr(feature = "dummy", derive(fake::Dummy))]
#[serde(rename_all = "camelCase")]
pub struct CreateRequest {
    pub platform_id: Id,
    pub platform_page_id: Id,
    pub connection_platform: String,
    pub connection_definition_id: Id,
    pub platform_version: String,
    pub model_name: String,
    pub schema: JsonSchema,
    pub sample: Value,
    pub paths: Option<SchemaPaths>,
    #[cfg_attr(feature = "dummy", dummy(default))]
    pub mapping: Option<Mappings>,
}

impl HookExt<ConnectionModelSchema> for CreateRequest {}

impl RequestExt for CreateRequest {
    type Output = ConnectionModelSchema;

    fn from(&self) -> Option<Self::Output> {
        let key = format!(
            "api::{}::{}::{}",
            self.connection_platform, self.platform_version, self.model_name
        )
        .to_lowercase();

        Some(Self::Output {
            id: Id::now(IdPrefix::ConnectionModelSchema),
            platform_id: self.platform_id,
            platform_page_id: self.platform_page_id,
            connection_platform: self.connection_platform.clone(),
            connection_definition_id: self.connection_definition_id,
            platform_version: self.platform_version.clone(),
            key,
            model_name: self.model_name.clone(),
            schema: self.schema.clone(),
            mapping: self.mapping.clone(),
            sample: self.sample.clone(),
            paths: self.paths.clone(),
            record_metadata: Default::default(),
        })
    }

    fn update(&self, record: &mut Self::Output) {
        record.platform_id = self.platform_id;
        record.platform_page_id = self.platform_page_id;
        record.connection_platform = self.connection_platform.clone();
        record.connection_definition_id = self.connection_definition_id;
        record.platform_version = self.platform_version.clone();
        record.model_name = self.model_name.clone();
        record.schema = self.schema.clone();
        record.sample = self.sample.clone();
        record.paths = self.paths.clone();
        record.mapping = self.mapping.clone();
    }

    fn get_store(stores: AppStores) -> MongoStore<Self::Output> {
        stores.model_schema.clone()
    }
}
