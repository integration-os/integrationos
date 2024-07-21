use super::{read, PublicExt, RequestExt};
use crate::server::{AppState, AppStores};
use axum::{routing::get, Router};
use bson::doc;
use integrationos_domain::{algebra::MongoStore, Event};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub fn get_router() -> Router<Arc<AppState>> {
    Router::new().route("/", get(read::<CreateEventRequest, Event>))
}

#[derive(Serialize, Deserialize)]
pub struct CreateEventRequest;

impl PublicExt<Event> for CreateEventRequest {
    fn public(input: Event) -> serde_json::Value {
        serde_json::to_value(input.to_public()).unwrap_or_default()
    }
}
impl RequestExt for CreateEventRequest {
    type Output = Event;

    fn get_store(stores: AppStores) -> MongoStore<Self::Output> {
        stores.event
    }
}
