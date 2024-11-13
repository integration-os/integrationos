use super::{create, delete, read, update, HookExt, PublicExt, RequestExt};
use crate::server::{AppState, AppStores};
use axum::{
    routing::{patch, post},
    Router,
};
use fake::Dummy;
use integrationos_domain::{
    algebra::MongoStore,
    common_model::CommonEnum,
    id::{prefix::IdPrefix, Id},
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub fn get_router() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/",
            post(create::<CreateRequest, CommonEnum>).get(read::<CreateRequest, CommonEnum>),
        )
        .route(
            "/:id",
            patch(update::<CreateRequest, CommonEnum>).delete(delete::<CreateRequest, CommonEnum>),
        )
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, Dummy)]
#[serde(rename_all = "camelCase")]
pub struct CreateRequest {
    #[serde(rename = "_id")]
    pub id: Option<Id>,
    pub name: String,
    pub options: Vec<String>,
}

impl PublicExt<CommonEnum> for CreateRequest {}
impl HookExt<CommonEnum> for CreateRequest {}

impl RequestExt for CreateRequest {
    type Output = CommonEnum;

    fn from(&self) -> Option<Self::Output> {
        let record = Self::Output {
            id: self.id.unwrap_or_else(|| Id::now(IdPrefix::CommonEnum)),
            name: self.name.clone(),
            options: self.options.clone(),
            record_metadata: Default::default(),
        };
        Some(record)
    }

    fn update(&self, mut record: Self::Output) -> Self::Output {
        record.name.clone_from(&self.name);
        record.options.clone_from(&self.options);
        record
    }

    fn get_store(stores: AppStores) -> MongoStore<Self::Output> {
        stores.common_enum.clone()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct GetRequest;

impl PublicExt<CommonEnum> for GetRequest {}
impl RequestExt for GetRequest {
    type Output = CommonEnum;

    fn get_store(stores: AppStores) -> MongoStore<Self::Output> {
        stores.common_enum
    }
}
