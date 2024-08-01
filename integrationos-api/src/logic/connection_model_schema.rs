use super::{create, delete, read, update, HookExt, PublicExt, ReadResponse, RequestExt};
use crate::{
    helper::shape_mongo_filter,
    router::ServerResponse,
    server::{AppState, AppStores},
};
use axum::{
    extract::{Path, Query, State},
    routing::{patch, post},
    Extension, Json, Router,
};
use futures::try_join;
use integrationos_domain::{
    algebra::MongoStore,
    connection_model_schema::{
        ConnectionModelSchema, Mappings, PublicConnectionModelSchema, SchemaPaths,
    },
    event_access::EventAccess,
    id::{prefix::IdPrefix, Id},
    json_schema::JsonSchema,
    ApplicationError, IntegrationOSError,
};
use mongodb::bson::doc;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::Value;
use std::{collections::BTreeMap, sync::Arc};
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

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[cfg_attr(feature = "dummy", derive(fake::Dummy))]
#[serde(rename_all = "camelCase")]
pub struct PublicGetConnectionModelSchema;

pub async fn public_get_connection_model_schema<T, U>(
    access: Option<Extension<Arc<EventAccess>>>,
    query: Option<Query<BTreeMap<String, String>>>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<ServerResponse<ReadResponse<U>>>, IntegrationOSError>
where
    T: RequestExt<Output = U>,
    U: Serialize + DeserializeOwned + Unpin + Sync + Send + 'static,
{
    match query.as_ref().and_then(|q| q.get("connectionDefinitionId")) {
        Some(id) => id.to_string(),
        None => {
            return Err(ApplicationError::bad_request(
                "connectionDefinitionId is required",
                None,
            ));
        }
    };

    let mut query = shape_mongo_filter(
        query,
        access.map(|e| {
            let Extension(e) = e;
            e
        }),
        None,
    );

    query.filter.remove("ownership.buildableId");
    query.filter.remove("environment");
    query.filter.insert("mapping", doc! { "$ne": null });

    let store = T::get_store(state.app_stores.clone());
    let count = store.count(query.filter.clone(), None);
    let find = store.get_many(
        Some(query.filter),
        None,
        None,
        Some(query.limit),
        Some(query.skip),
    );

    let res = match try_join!(count, find) {
        Ok((total, rows)) => ReadResponse {
            rows,
            skip: query.skip,
            limit: query.limit,
            total,
        },
        Err(e) => {
            error!("Error reading from store: {e}");
            return Err(e);
        }
    };

    Ok(Json(ServerResponse::new("connection_model_schema", res)))
}

impl RequestExt for PublicGetConnectionModelSchema {
    type Output = PublicConnectionModelSchema;

    fn get_store(stores: AppStores) -> MongoStore<Self::Output> {
        stores.public_model_schema.clone()
    }
}

pub async fn get_platform_models(
    Path(platform_name): Path<String>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<String>>, IntegrationOSError> {
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
            e
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
    pub _id: Option<Id>,
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
impl PublicExt<ConnectionModelSchema> for CreateRequest {}

impl RequestExt for CreateRequest {
    type Output = ConnectionModelSchema;

    fn from(&self) -> Option<Self::Output> {
        let key = format!(
            "api::{}::{}::{}",
            self.connection_platform, self.platform_version, self.model_name
        )
        .to_lowercase();

        Some(Self::Output {
            id: self
                ._id
                .unwrap_or_else(|| Id::now(IdPrefix::ConnectionModelSchema)),
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

    fn update(&self, mut record: Self::Output) -> Self::Output {
        record.platform_id = self.platform_id;
        record.platform_page_id = self.platform_page_id;
        record
            .connection_platform
            .clone_from(&self.connection_platform);
        record.connection_definition_id = self.connection_definition_id;
        record.platform_version.clone_from(&self.platform_version);
        record.model_name.clone_from(&self.model_name);
        record.schema = self.schema.clone();
        record.sample = self.sample.clone();
        record.paths.clone_from(&self.paths);
        record.mapping.clone_from(&self.mapping);
        record
    }

    fn get_store(stores: AppStores) -> MongoStore<Self::Output> {
        stores.model_schema.clone()
    }
}
