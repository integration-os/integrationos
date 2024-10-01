use super::{
    connection::{create_connection, delete_connection, update_connection},
    read, PublicExt, RequestExt,
};
use crate::server::{AppState, AppStores};
use axum::{
    routing::{delete, get, patch, post},
    Router,
};
use integrationos_domain::{algebra::MongoStore, id::Id, ConnectionIdentityType, PublicConnection};
use mongodb::bson::doc;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};
use validator::Validate;

pub fn get_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", post(create_connection))
        .route(
            "/",
            get(read::<CreatePublicConnectionPayload, PublicConnection>),
        )
        .route("/:id", patch(update_connection))
        .route("/:id", delete(delete_connection))
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct CreatePublicConnectionPayload {
    pub connection_definition_id: Id,
    pub auth_form_data: HashMap<String, String>,
    pub active: bool,
    pub identity: Option<String>,
    pub identity_type: Option<ConnectionIdentityType>,
}

impl PublicExt<PublicConnection> for CreatePublicConnectionPayload {}

impl RequestExt for CreatePublicConnectionPayload {
    type Output = PublicConnection;

    fn get_store(stores: AppStores) -> MongoStore<Self::Output> {
        stores.public_connection
    }
}
