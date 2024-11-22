use crate::server::AppState;
use axum::response::IntoResponse;
use futures_util::StreamExt;
use http::{HeaderName, HeaderValue, Request};
use integrationos_domain::{ApplicationError, Store};
use mongodb::options::FindOptions;
use serde::Deserialize;
use std::{
    collections::BTreeSet,
    error::Error,
    fmt::Display,
    sync::{Arc, RwLock},
    time::Duration,
};
use tower::{filter::Predicate, BoxError};
use tracing::{error, trace};

pub type Whitelist = Arc<RwLock<BTreeSet<HeaderValue>>>;

#[derive(Debug, Clone)]
pub struct BlockInvalidHeaders {
    whitelist: Whitelist,
    header_name: HeaderName,
}

impl BlockInvalidHeaders {
    pub async fn from_state(state: Arc<AppState>) -> Self {
        let whitelist = Arc::new(RwLock::new(BTreeSet::new()));

        let header_name =
            HeaderName::from_lowercase(state.config.headers.auth_header.as_bytes()).unwrap();

        let (tx, mut rx) = tokio::sync::mpsc::channel::<()>(1);

        let whitelist_clone = whitelist.clone();
        tokio::spawn(async move {
            loop {
                #[derive(Deserialize)]
                struct SparseEventAccess {
                    #[serde(with = "http_serde_ext::header_value", rename = "accessKey")]
                    access_key: HeaderValue,
                }

                let mut records = match state
                    .app_stores
                    .db
                    .collection::<SparseEventAccess>(&Store::EventAccess.to_string())
                    .find(bson::doc! { "deleted": false })
                    .with_options(
                        FindOptions::builder()
                            .projection(bson::doc! {
                               "accessKey": 1
                            })
                            .build(),
                    )
                    .await
                {
                    Err(e) => {
                        error!("Could not fetch event access records cursor: {e}");
                        continue;
                    }
                    Ok(records) => records,
                };

                #[allow(clippy::mutable_key_type)]
                let mut new_whitelist = BTreeSet::new();
                while let Some(result) = records.next().await {
                    match result {
                        Ok(record) => {
                            let mut header_value = record.access_key;
                            header_value.set_sensitive(true);

                            new_whitelist.insert(header_value);
                        }

                        Err(e) => {
                            error!("Could not fetch event access record: {e}");
                            continue;
                        }
                    }
                }

                let len = new_whitelist.len();

                {
                    let mut whitelist_clone = whitelist_clone.write().unwrap();
                    *whitelist_clone = new_whitelist
                }

                trace!("Updated whitelist with {len} entries");

                let _ = tx.send(()).await;
                tokio::time::sleep(Duration::from_secs(
                    state.config.access_key_whitelist_refresh_interval_secs,
                ))
                .await;
            }
        });

        rx.recv().await;

        BlockInvalidHeaders {
            whitelist,
            header_name,
        }
    }
}

#[derive(Debug)]
struct FastError;

impl Display for FastError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Error")
    }
}

impl Error for FastError {}

impl<T> Predicate<Request<T>> for BlockInvalidHeaders {
    type Request = Request<T>;

    fn check(&mut self, request: Request<T>) -> Result<Self::Request, BoxError> {
        let Some(header_value) = request.headers().get(&self.header_name) else {
            return Err(Box::new(FastError));
        };

        {
            let whitelist = self.whitelist.read().unwrap();
            if !whitelist.contains(header_value) {
                return Err(Box::new(FastError));
            }
        }

        Ok(request)
    }
}

pub async fn handle_blocked_error(_: BoxError) -> impl IntoResponse {
    ApplicationError::unauthorized("You are not authorized to access this resource", None)
}
