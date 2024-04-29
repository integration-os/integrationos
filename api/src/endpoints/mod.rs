use crate::{
    api_payloads::ErrorResponse,
    internal_server_error, not_found,
    server::{AppState, AppStores},
    util::shape_mongo_filter,
};
use anyhow::Result;
use axum::{
    async_trait,
    extract::{Path, Query, State},
    Extension, Json,
};
use bson::{doc, SerializerOptions};
use http::{HeaderMap, HeaderValue, StatusCode};
use integrationos_domain::{
    algebra::{MongoStore, StoreExt},
    event_access::EventAccess,
    ApplicationError, Connection, IntegrationOSError, InternalError, OAuth, Store,
};
use moka::future::Cache;
use mongodb::options::FindOneOptions;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{collections::BTreeMap, fmt::Debug, sync::Arc};
use tokio::try_join;
use tracing::error;

pub mod common_enum;
pub mod common_model;
pub mod connection;
pub mod connection_definition;
pub mod connection_model_definition;
pub mod connection_model_schema;
pub mod connection_oauth_definition;
pub mod event_access;
pub mod events;
pub mod metrics;
pub mod oauth;
pub mod openapi;
pub mod passthrough;
pub mod pipeline;
pub mod transactions;
pub mod unified;

const INTEGRATION_OS_PASSTHROUGH_HEADER: &str = "x-integrationos-passthrough";

pub type Unit = ();
pub trait CrudRequest: Sized {
    type Output: Serialize + DeserializeOwned + Unpin + Sync + Send + 'static;

    /// Generate the output of the request based on the input and the event access.
    ///
    /// @param self
    /// @param event_access
    /// @return Option<Self::Output>
    fn event_access(&self, _: Arc<EventAccess>) -> Option<Self::Output> {
        None
    }

    /// Generate the output of the request based on the input.
    /// @param self
    /// @return Result<Option<Self::Output>, Self::Error>
    fn output(&self) -> Option<Self::Output> {
        None
    }

    /// Update the output of the request based on the input.
    fn update(&self, _: &mut Self::Output) -> Unit {}

    /// Get the store for the request.
    fn get_store(stores: AppStores) -> MongoStore<Self::Output>;
}

pub trait CachedRequest: CrudRequest {
    fn get_cache(
        state: Arc<AppState>,
    ) -> Arc<Cache<Option<BTreeMap<String, String>>, Arc<ReadResponse<Self::Output>>>>;
}

pub type ApiError = (StatusCode, Json<ErrorResponse>);
pub type ApiResult<T> = Result<Json<T>, ApiError>;

#[async_trait]
pub trait CrudHook<Input>
where
    Input: Serialize + DeserializeOwned + Unpin + Sync + Send + 'static,
{
    async fn after_create_hook(
        _record: &Input,
        _stores: &AppStores,
    ) -> Result<(), IntegrationOSError> {
        Ok(())
    }

    async fn after_update_hook(
        _record: &Input,
        _stores: &AppStores,
    ) -> Result<(), IntegrationOSError> {
        Ok(())
    }

    async fn after_delete_hook(
        _record: &Input,
        _stores: &AppStores,
    ) -> Result<(), IntegrationOSError> {
        Ok(())
    }

    async fn after_read_hook(
        _record: &Input,
        _stores: &AppStores,
    ) -> Result<(), IntegrationOSError> {
        Ok(())
    }
}

pub async fn create<T, U>(
    event_access: Option<Extension<Arc<EventAccess>>>,
    State(state): State<Arc<AppState>>,
    Json(req): Json<T>,
) -> ApiResult<U>
where
    T: CrudRequest<Output = U> + CrudHook<U> + 'static,
    U: Serialize + DeserializeOwned + Unpin + Sync + Send + Debug + 'static,
{
    let output = event_access
        .map_or_else(
            || req.output(),
            |event_access| req.event_access(event_access.0).or(req.output()),
        )
        .ok_or_else(|| not_found!("Record"))?;

    match T::get_store(state.app_stores.clone())
        .create_one(&output)
        .await
    {
        Ok(_) => {
            T::after_create_hook(&output, &state.app_stores)
                .await
                .map_err(|e| {
                    error!("Error running after create hook: {:?}", e);
                })
                .ok();

            Ok(Json(output))
        }
        Err(e) => {
            error!("Error creating object: {e}");
            Err(internal_server_error!())
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct ReadResponse<T> {
    pub rows: Vec<T>,
    pub total: u64,
    pub skip: u64,
    pub limit: u64,
}

pub async fn read<T, U>(
    headers: HeaderMap,
    event_access: Option<Extension<Arc<EventAccess>>>,
    query: Option<Query<BTreeMap<String, String>>>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<ReadResponse<U>>, ApiError>
where
    T: CrudRequest<Output = U> + 'static,
    U: Serialize + DeserializeOwned + Unpin + Sync + Send + Debug + 'static,
{
    let query = shape_mongo_filter(
        query,
        event_access.map(|e| {
            let Extension(e) = e;
            e
        }),
        Some(headers),
    );

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
            println!("{:?}", e);
            return Err(internal_server_error!());
        }
    };

    Ok(Json(res))
}

pub async fn read_cached<T, U>(
    query: Option<Query<BTreeMap<String, String>>>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<Arc<ReadResponse<U>>>, ApiError>
where
    T: CachedRequest<Output = U> + 'static,
    U: Clone + Serialize + DeserializeOwned + Unpin + Sync + Send + Debug + 'static,
{
    let cache = T::get_cache(state.clone());

    let res = cache
        .try_get_with(query.as_ref().map(|q| q.0.clone()), async {
            let query = shape_mongo_filter(query, None, None);

            println!("{:?}", query);
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
                Ok((total, rows)) => {
                    println!("{:?}", total);

                    Arc::new(ReadResponse {
                        rows,
                        skip: query.skip,
                        limit: query.limit,
                        total,
                    })
                }
                Err(e) => {
                    error!("Error reading from store: {e}");
                    return Err(internal_server_error!());
                }
            };

            Ok(res)
        })
        .await
        .map_err(Arc::unwrap_or_clone)?;

    Ok(Json(res))
}

#[derive(Serialize, Deserialize)]
pub struct SuccessResponse {
    success: bool,
}

pub async fn update<T, U>(
    event_access: Option<Extension<Arc<EventAccess>>>,
    Path(id): Path<String>,
    State(state): State<Arc<AppState>>,
    Json(req): Json<T>,
) -> Result<Json<SuccessResponse>, ApiError>
where
    T: CrudRequest<Output = U> + CrudHook<U> + 'static,
    U: Serialize + DeserializeOwned + Unpin + Sync + Send + 'static,
{
    let mut query = shape_mongo_filter(
        None,
        event_access.map(|e| {
            let Extension(e) = e;
            e
        }),
        None,
    );
    query.filter.insert("_id", id.clone());

    let store = T::get_store(state.app_stores.clone());

    let Some(mut record) = (match store.get_one(query.filter).await {
        Ok(ret) => ret,
        Err(e) => {
            error!("Error getting record in store: {e}");
            return Err(internal_server_error!());
        }
    }) else {
        return Err(not_found!("Record"));
    };

    req.update(&mut record);

    let bson = bson::to_bson_with_options(
        &record,
        SerializerOptions::builder().human_readable(false).build(),
    )
    .map_err(|e| {
        error!("Could not serialize record into document: {e}");
        internal_server_error!()
    })?;

    let document = doc! {
        "$set": bson
    };

    match store.update_one(&id, document).await {
        Ok(_) => {
            T::after_update_hook(&record, &state.app_stores)
                .await
                .map_err(|e| {
                    error!("Error running after update hook: {:?}", e);
                })
                .ok();
            Ok(Json(SuccessResponse { success: true }))
        }
        Err(e) => {
            error!("Error updating in store: {e}");
            Err(internal_server_error!())
        }
    }
}

pub async fn delete<T, U>(
    event_access: Option<Extension<Arc<EventAccess>>>,
    Path(id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> ApiResult<U>
where
    T: CrudRequest<Output = U> + 'static,
    U: Serialize + DeserializeOwned + Unpin + Sync + Send + 'static,
{
    let store = T::get_store(state.app_stores.clone());

    let mut query = shape_mongo_filter(
        None,
        event_access.map(|e| {
            let Extension(e) = e;
            e
        }),
        None,
    );
    query.filter.insert("_id", id.clone());

    let Some(res) = (match store.get_one(query.filter).await {
        Ok(ret) => ret,
        Err(e) => {
            error!("Could not get record from store: {e}");
            return Err(internal_server_error!());
        }
    }) else {
        return Err(not_found!("Record"));
    };

    match store
        .update_one(
            &id,
            doc! {
                "$set": {
                    "deleted": true,
                }
            },
        )
        .await
    {
        Ok(_) => Ok(Json(res)),
        Err(e) => {
            error!("Could not update record in store: {e}");
            Err(internal_server_error!())
        }
    }
}

#[derive(Deserialize)]
struct SparseConnection {
    oauth: OAuth,
}

async fn get_connection(
    access: &EventAccess,
    connection_id: &HeaderValue,
    stores: &AppStores,
    cache: &Cache<(Arc<str>, HeaderValue), Arc<Connection>>,
) -> Result<Arc<Connection>, IntegrationOSError> {
    let connection = cache
        .try_get_with(
            (access.ownership.id.clone(), connection_id.clone()),
            async {
                let Ok(connection_id_str) = connection_id.to_str() else {
                    return Err(ApplicationError::bad_request(
                        "Invalid connection key header",
                        None,
                    ));
                };

                let connection = match stores
                    .connection
                    .get_one(doc! {
                        "key": connection_id_str,
                        "ownership.buildableId": access.ownership.id.as_ref(),
                        "deleted": false
                    })
                    .await
                {
                    Ok(Some(data)) => Arc::new(data),
                    Ok(None) => {
                        return Err(ApplicationError::not_found("Connection", None));
                    }
                    Err(e) => {
                        error!("Error fetching connection: {:?}", e);

                        return Err(InternalError::unknown("Error fetching connection", None));
                    }
                };

                Ok(connection)
            },
        )
        .await
        .map_err(Arc::unwrap_or_clone)?;

    if let Some(OAuth::Enabled { .. }) = connection.oauth {
        let sparse_connection = match stores
            .db
            .collection::<SparseConnection>(&Store::Connections.to_string())
            .find_one(
                doc! {
                    "_id": &connection.id.to_string(),
                    "ownership.buildableId": access.ownership.id.as_ref(),
                    "deleted": false
                },
                FindOneOptions::builder()
                    .projection(doc! {
                       "oauth": 1
                    })
                    .build(),
            )
            .await
        {
            Ok(Some(data)) => data,
            Ok(None) => {
                return Err(ApplicationError::not_found("Connection", None));
            }
            Err(e) => {
                error!("Error fetching connection: {:?}", e);

                return Err(InternalError::unknown("Error fetching connection", None));
            }
        };
        let mut connection = (*connection).clone();
        connection.oauth = Some(sparse_connection.oauth);
        return Ok(Arc::new(connection));
    }
    Ok(connection)
}
