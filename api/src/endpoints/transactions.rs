use std::sync::Arc;

use axum::{routing::get, Router};
use bson::doc;
use integrationos_domain::common::{event_access::EventAccess, mongo::MongoDbStore, Transaction};
use serde::{Deserialize, Serialize};

use crate::server::{AppState, AppStores};

use super::{read, CrudRequest};

pub fn get_router() -> Router<Arc<AppState>> {
    Router::new().route("/", get(read::<TransactionCrud, Transaction>))
}

#[derive(Serialize, Deserialize)]
pub struct TransactionCrud;

impl CrudRequest for TransactionCrud {
    type Output = Transaction;
    type Error = ();

    fn into_public(self) -> anyhow::Result<Self::Output, Self::Error> {
        unimplemented!()
    }

    fn into_with_event_access(self, _event_access: Arc<EventAccess>) -> Self::Output {
        unimplemented!()
    }

    fn update(self, _record: &mut Self::Output) {
        unimplemented!()
    }

    fn get_store(stores: AppStores) -> MongoDbStore<Self::Output> {
        stores.transactions
    }
}
