use super::{create, delete, read, update, CrudHook, CrudRequest};
use crate::server::{AppState, AppStores};
use axum::{
    routing::{patch, post},
    Router,
};
use integrationos_domain::{
    algebra::MongoStore,
    id::{prefix::IdPrefix, Id},
    ownership::Owners,
    PlatformData,
};
use mongodb::bson::doc;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub fn get_router() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/",
            post(create::<CreateRequest, PlatformData>).get(read::<CreateRequest, PlatformData>),
        )
        .route(
            "/:id",
            patch(update::<CreateRequest, PlatformData>)
                .delete(delete::<CreateRequest, PlatformData>),
        )
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[cfg_attr(feature = "dummy", derive(fake::Dummy))]
#[serde(rename_all = "camelCase")]
pub struct CreateRequest {
    pub connection_definition_id: Id,
    pub name: String,
    pub url: String,
    pub version: String,
    pub ownership: Owners,
    pub analyzed: bool,
}

impl CrudHook<PlatformData> for CreateRequest {}

impl CrudRequest for CreateRequest {
    type Output = PlatformData;

    fn output(&self) -> Option<Self::Output> {
        Some(Self::Output {
            id: Id::now(IdPrefix::Platform),
            connection_definition_id: self.connection_definition_id,
            name: self.name.clone(),
            url: self.url.clone(),
            platform_version: self.version.clone(),
            ownership: self.ownership.clone(),
            analyzed: self.analyzed,
            record_metadata: Default::default(),
        })
    }

    fn update(&self, record: &mut Self::Output) {
        record.connection_definition_id = self.connection_definition_id;
        record.name = self.name.clone();
        record.url = self.url.clone();
        record.platform_version = self.version.clone();
        record.ownership = self.ownership.clone();
        record.analyzed = self.analyzed;
    }

    fn get_store(stores: AppStores) -> MongoStore<Self::Output> {
        stores.platform.clone()
    }
}
