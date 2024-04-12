use super::{read, CrudRequest};
use crate::server::{AppState, AppStores};
use axum::{routing::get, Router};
use bson::doc;
use integrationos_domain::{
    algebra::MongoStore,
    common::{event_access::EventAccess, Event},
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub fn get_router() -> Router<Arc<AppState>> {
    Router::new().route("/", get(read::<CreateEventRequest, Event>))
}

#[derive(Serialize, Deserialize)]
pub struct CreateEventRequest;

impl CrudRequest for CreateEventRequest {
    type Output = Event;
    type Error = ();

    fn into_with_event_access(self, _event_access: Arc<EventAccess>) -> Self::Output {
        unimplemented!()
    }

    fn update(self, _record: &mut Self::Output) {
        unimplemented!()
    }

    fn get_store(stores: AppStores) -> MongoStore<Self::Output> {
        stores.event
    }

    fn into_public(self) -> anyhow::Result<Self::Output, Self::Error> {
        unimplemented!()
    }
}
