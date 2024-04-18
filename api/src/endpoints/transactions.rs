use super::{read, CrudRequest};
use crate::server::{AppState, AppStores};
use axum::{routing::get, Router};
use bson::doc;
use integrationos_domain::{algebra::MongoStore, Transaction};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub fn get_router() -> Router<Arc<AppState>> {
    Router::new().route("/", get(read::<TransactionCrud, Transaction>))
}

#[derive(Serialize, Deserialize)]
pub struct TransactionCrud;

impl CrudRequest for TransactionCrud {
    type Output = Transaction;

    fn get_store(stores: AppStores) -> MongoStore<Self::Output> {
        stores.transactions
    }
}
