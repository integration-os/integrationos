use super::{create, delete, read, update, ApiResult, CrudHook, CrudRequest};
use crate::{
    internal_server_error, not_found,
    server::{AppState, AppStores},
};
use axum::{
    async_trait,
    extract::{Json, Path, State},
    routing::{get, patch, post},
    Router,
};
use integrationos_domain::{
    algebra::{MongoStore, StoreExt},
    api_model_config::Lang,
    common::{
        common_model::{CommonModel, Field},
        event_access::EventAccess,
        json_schema::JsonSchema,
    },
    id::{prefix::IdPrefix, Id},
    IntegrationOSError,
};
use mongodb::bson::doc;
use semver::Version;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{collections::HashMap, sync::Arc};
use tracing::error;

pub fn get_router() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/",
            post(create::<CreateRequest, CommonModel>).get(read::<CreateRequest, CommonModel>),
        )
        .route(
            "/:id",
            patch(update::<CreateRequest, CommonModel>)
                .delete(delete::<CreateRequest, CommonModel>),
        )
        .route("/:id/schema", get(as_json_schema))
        .route("/:id/expand", get(expand))
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[cfg_attr(feature = "dummy", derive(fake::Dummy))]
#[serde(rename_all = "camelCase")]
pub struct CreateRequest {
    pub name: String,
    pub version: Version,
    pub fields: Vec<Field>,
    pub category: String,
    pub sample: Value,
    pub primary: bool,
}

#[async_trait]
impl CrudHook<CommonModel> for CreateRequest {
    async fn after_create_hook(
        record: &CommonModel,
        stores: &AppStores,
    ) -> Result<(), IntegrationOSError> {
        let rust = record.generate_as(&Lang::Rust);
        let typescript = record.generate_as(&Lang::TypeScript);
        let interface =
            HashMap::from_iter(vec![(Lang::Rust, rust), (Lang::TypeScript, typescript)]);

        update_interface(interface, record, &stores.common_model).await
    }

    async fn after_update_hook(
        record: &CommonModel,
        stores: &AppStores,
    ) -> Result<(), IntegrationOSError> {
        let typescript = record.generate_as(&Lang::TypeScript);
        let rust = record.generate_as(&Lang::Rust);
        let interface =
            HashMap::from_iter(vec![(Lang::Rust, rust), (Lang::TypeScript, typescript)]);

        update_interface(interface, record, &stores.common_model).await
    }
}

impl CrudRequest for CreateRequest {
    type Output = CommonModel;
    type Error = ();

    fn into_public(self) -> Result<Self::Output, Self::Error> {
        let mut record = Self::Output {
            id: Id::now(IdPrefix::CommonModel),
            name: self.name,
            fields: self.fields,
            sample: self.sample,
            category: self.category,
            primary: self.primary,
            interface: Default::default(),
            record_metadata: Default::default(),
        };
        record.record_metadata.version = self.version;
        Ok(record)
    }

    fn into_with_event_access(self, _event_access: Arc<EventAccess>) -> Self::Output {
        unimplemented!()
    }

    fn update(self, record: &mut Self::Output) {
        record.name = self.name;
        record.record_metadata.version = self.version;
        record.fields = self.fields;
        record.category = self.category;
        record.sample = self.sample;
    }

    fn get_store(stores: AppStores) -> MongoStore<Self::Output> {
        stores.common_model.clone()
    }
}

async fn expand(Path(id): Path<Id>, State(state): State<Arc<AppState>>) -> ApiResult<CommonModel> {
    let Some(cm) = state
        .app_stores
        .common_model
        .get_one_by_id(&id.to_string())
        .await
        .map_err(|e| {
            error!("Could not fetch common model: {e}");
            internal_server_error!()
        })?
    else {
        return Err(not_found!("Common model"));
    };

    let expanded = cm
        .expand_all(
            state.app_stores.common_model.clone(),
            state.app_stores.common_enum.clone(),
        )
        .await
        .map_err(|e| {
            error!("Could not expand all: {e}");
            internal_server_error!()
        })?;

    Ok(Json(expanded))
}

async fn as_json_schema(path: Path<Id>, state: State<Arc<AppState>>) -> ApiResult<JsonSchema> {
    let Json(cm) = expand(path, state).await?;

    match CommonModel::try_into(cm) {
        Ok(schema) => Ok(Json(schema)),
        Err(e) => {
            error!("Could not convert to json schema: {e}");
            Err(internal_server_error!())
        }
    }
}

async fn update_interface(
    interface: HashMap<Lang, String>,
    record: &CommonModel,
    cm_store: &MongoStore<CommonModel>,
) -> Result<(), IntegrationOSError> {
    match bson::to_bson(&interface) {
        Ok(interface) => {
            cm_store
                .update_one(
                    &record.id.to_string(),
                    doc! {"$set": {"interface": interface}},
                )
                .await
                .ok();

            Ok(())
        }
        Err(e) => {
            error!("Could not convert interface to bson: {e}");
            Ok(())
        }
    }
}
