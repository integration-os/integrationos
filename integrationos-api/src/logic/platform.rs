use super::{create, delete, read, update, HookExt, PublicExt, RequestExt};
use crate::server::{AppState, AppStores};
use axum::{
    routing::{patch, post},
    Router,
};
use fake::Dummy;
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

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize, Dummy)]
#[serde(rename_all = "camelCase")]
pub struct CreateRequest {
    #[serde(rename = "_id")]
    pub id: Option<Id>,
    pub connection_definition_id: Id,
    pub name: String,
    pub url: String,
    pub version: String,
    pub ownership: Owners,
    pub analyzed: bool,
}

impl HookExt<PlatformData> for CreateRequest {}
impl PublicExt<PlatformData> for CreateRequest {}

impl RequestExt for CreateRequest {
    type Output = PlatformData;

    fn from(&self) -> Option<Self::Output> {
        Some(Self::Output {
            id: self.id.unwrap_or_else(|| Id::now(IdPrefix::Platform)),
            connection_definition_id: self.connection_definition_id,
            name: self.name.clone(),
            url: self.url.clone(),
            platform_version: self.version.clone(),
            ownership: self.ownership.clone(),
            analyzed: self.analyzed,
            record_metadata: Default::default(),
        })
    }

    fn update(&self, mut record: Self::Output) -> Self::Output {
        record.connection_definition_id = self.connection_definition_id;
        record.name.clone_from(&self.name);
        record.url.clone_from(&self.url);
        record.platform_version.clone_from(&self.version);
        record.ownership = self.ownership.clone();
        record.analyzed = self.analyzed;

        record
    }

    fn get_store(stores: AppStores) -> MongoStore<Self::Output> {
        stores.platform.clone()
    }
}
