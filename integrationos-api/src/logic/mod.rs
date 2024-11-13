use crate::{
    helper::shape_mongo_filter,
    router::ServerResponse,
    server::{AppState, AppStores},
};
use axum::{
    extract::{Path, Query, State},
    Extension, Json,
};
use bson::doc;
use http::{HeaderMap, HeaderValue};
use integrationos_cache::local::connection_cache::ConnectionCacheArcStrHeaderKey;
use integrationos_domain::{
    algebra::MongoStore, event_access::EventAccess, ApplicationError, Connection,
    IntegrationOSError, InternalError, OAuth, Store, Unit,
};
use mongodb::options::FindOneOptions;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::Value;
use std::{collections::BTreeMap, fmt::Debug, future::Future, sync::Arc};
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
pub mod event_callback;
pub mod events;
pub mod metrics;
pub mod oauth;
pub mod openapi;
pub mod passthrough;
pub mod pipeline;
pub mod platform;
pub mod platform_page;
pub mod schema_generator;
pub mod secrets;
pub mod transactions;
pub mod unified;
pub mod utils;
pub mod vault_connection;

const INTEGRATION_OS_PASSTHROUGH_HEADER: &str = "x-integrationos-passthrough";

pub trait RequestExt: Sized {
    type Output: Serialize + DeserializeOwned + Unpin + Sync + Send + 'static;
    /// Generate `Self::Output` of the request based on the given payload.
    ///
    /// @param self
    /// @return Result<Option<Self::Output>, Self::Error>
    fn from(&self) -> Option<Self::Output> {
        None
    }

    /// Generate `Self::Output` of the request based on the passed event access.
    ///
    /// @param self
    /// @param event_access
    /// @return Option<Self::Output>
    fn access(&self, _: Arc<EventAccess>) -> Option<Self::Output> {
        None
    }

    fn update(&self, output: Self::Output) -> Self::Output {
        output
    }

    fn get_store(stores: AppStores) -> MongoStore<Self::Output>;
}

pub trait HookExt<Input>
where
    Input: Serialize + DeserializeOwned + Unpin + Sync + Send + 'static,
{
    fn after_create_hook(
        _record: &Input,
        _stores: &AppStores,
    ) -> impl std::future::Future<Output = Result<Unit, IntegrationOSError>> + Send {
        async { Ok(()) }
    }

    fn after_update_hook(
        _record: &Input,
        _stores: &AppStores,
    ) -> impl std::future::Future<Output = Result<Unit, IntegrationOSError>> + Send {
        async { Ok(()) }
    }

    fn after_delete_hook(
        _record: &Input,
        _stores: &AppStores,
    ) -> impl std::future::Future<Output = Result<Unit, IntegrationOSError>> + Send {
        async { Ok(()) }
    }

    fn after_read_hook(
        _record: &Input,
        _stores: &AppStores,
    ) -> impl Future<Output = Result<Unit, IntegrationOSError>> {
        async { Ok(()) }
    }
}

pub trait PublicExt<Input>
where
    Input: Serialize + DeserializeOwned + Unpin + Sync + Send + 'static,
{
    fn public(input: Input) -> Value {
        serde_json::to_value(input).unwrap_or_default()
    }
}

pub async fn create<T, U>(
    access: Option<Extension<Arc<EventAccess>>>,
    State(state): State<Arc<AppState>>,
    Json(payload): Json<T>,
) -> Result<Json<ServerResponse<Value>>, IntegrationOSError>
where
    T: RequestExt<Output = U> + HookExt<U> + PublicExt<U> + 'static,
    U: Serialize + DeserializeOwned + Unpin + Sync + Send + Debug + 'static,
{
    let output = access
        .map(|e| payload.access(e.0))
        .unwrap_or_else(|| payload.from())
        .ok_or_else(|| {
            error!("Could not generate output from payload");
            ApplicationError::bad_request("Could not generate output from payload", None)
        })?;

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

            Ok(Json(ServerResponse::new("create", T::public(output))))
        }
        Err(e) => {
            error!("Error creating object: {e}");
            Err(e)
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq, Default)]
pub struct ReadResponse<T> {
    pub rows: Vec<T>,
    pub total: u64,
    pub skip: u64,
    pub limit: u64,
}

pub async fn read<T, U>(
    headers: HeaderMap,
    access: Option<Extension<Arc<EventAccess>>>,
    query: Option<Query<BTreeMap<String, String>>>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<ServerResponse<ReadResponse<Value>>>, IntegrationOSError>
where
    T: RequestExt<Output = U> + PublicExt<U> + 'static,
    U: Serialize + DeserializeOwned + Unpin + Sync + Send + Debug + 'static,
{
    read_common::<T, U>(headers, access, query, State(state), true).await
}

pub async fn read_without_count<T, U>(
    headers: HeaderMap,
    access: Option<Extension<Arc<EventAccess>>>,
    query: Option<Query<BTreeMap<String, String>>>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<ServerResponse<ReadResponse<Value>>>, IntegrationOSError>
where
    T: RequestExt<Output = U> + PublicExt<U> + 'static,
    U: Serialize + DeserializeOwned + Unpin + Sync + Send + Debug + 'static,
{
    read_common::<T, U>(headers, access, query, State(state), false).await
}

#[derive(Serialize, Deserialize)]
pub struct SuccessResponse {
    success: bool,
}

pub async fn update<T, U>(
    access: Option<Extension<Arc<EventAccess>>>,
    Path(id): Path<String>,
    State(state): State<Arc<AppState>>,
    Json(payload): Json<T>,
) -> Result<Json<ServerResponse<SuccessResponse>>, IntegrationOSError>
where
    T: RequestExt<Output = U> + HookExt<U> + 'static,
    U: Serialize + DeserializeOwned + Unpin + Sync + Send + 'static,
{
    let mut query = shape_mongo_filter(
        None,
        access.map(|e| {
            let Extension(e) = e;
            e
        }),
        None,
    );
    query.filter.insert("_id", id.clone());

    let store = T::get_store(state.app_stores.clone());

    let Some(record) = (match store.get_one(query.filter).await {
        Ok(ret) => ret,
        Err(e) => {
            error!("Error getting record in store: {e}");
            return Err(e);
        }
    }) else {
        return Err(ApplicationError::not_found(
            &format!("Record with id {id} not found"),
            None,
        ));
    };

    let record = payload.update(record);

    let bson = bson::to_bson_with_options(&record, Default::default()).map_err(|e| {
        error!("Could not serialize record into document: {e}");
        InternalError::serialize_error(e.to_string().as_str(), None)
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
            Ok(Json(ServerResponse::new(
                "update",
                SuccessResponse { success: true },
            )))
        }
        Err(e) => {
            error!("Error updating in store: {e}");
            Err(e)
        }
    }
}

pub async fn delete<T, U>(
    event_access: Option<Extension<Arc<EventAccess>>>,
    Path(id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<ServerResponse<U>>, IntegrationOSError>
where
    T: RequestExt<Output = U> + 'static,
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
            return Err(e);
        }
    }) else {
        return Err(ApplicationError::not_found(
            &format!("Record with id {id} not found"),
            None,
        ));
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
        Ok(_) => Ok(Json(ServerResponse::new("delete", res))),
        Err(e) => {
            error!("Could not update record in store: {e}");
            Err(e)
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SparseConnection {
    oauth: OAuth,
    secrets_service_id: String,
}

async fn get_connection(
    access: &EventAccess,
    connection_key: &HeaderValue,
    stores: &AppStores,
    cache: &ConnectionCacheArcStrHeaderKey,
) -> Result<Arc<Connection>, IntegrationOSError> {
    let connection = cache
        .get_or_insert_with_filter(
            (access.ownership.id.clone(), connection_key.clone()),
            stores.connection.clone(),
            doc! {
                "key": connection_key.to_str().map_err(|_| {
                    ApplicationError::bad_request("Invalid connection key header", None)
                })?,
                "ownership.buildableId": access.ownership.id.as_ref(),
                "deleted": false
            },
        )
        .await?;

    // If Oauth is enabled, fetching the latest secret (due to refresh, cache can't be used)
    if let Some(OAuth::Enabled { .. }) = connection.oauth {
        let collection = stores
            .db
            .collection::<SparseConnection>(&Store::Connections.to_string());
        let filter = doc! {
            "key": &connection.key.to_string(),
            "ownership.buildableId": access.ownership.id.as_ref(),
            "deleted": false
        };
        let options = FindOneOptions::builder()
            .projection(doc! {
                "oauth": 1,
                "secretsServiceId": 1
            })
            .build();

        let sparse_connection = match collection.find_one(filter, options).await {
            Ok(Some(data)) => data,
            Ok(None) => return Err(ApplicationError::not_found("Connection", None)),
            Err(e) => {
                error!("Error fetching connection: {:?}", e);
                return Err(InternalError::unknown("Error fetching connection", None));
            }
        };

        let mut updated_connection = connection.clone();
        updated_connection.oauth = Some(sparse_connection.oauth);
        updated_connection.secrets_service_id = sparse_connection.secrets_service_id;

        return Ok(Arc::new(updated_connection));
    }
    Ok(Arc::new(connection))
}

pub async fn read_common<T, U>(
    headers: HeaderMap,
    access: Option<Extension<Arc<EventAccess>>>,
    query: Option<Query<BTreeMap<String, String>>>,
    State(state): State<Arc<AppState>>,
    count: bool,
) -> Result<Json<ServerResponse<ReadResponse<Value>>>, IntegrationOSError>
where
    T: RequestExt<Output = U> + PublicExt<U> + 'static,
    U: Serialize + DeserializeOwned + Unpin + Sync + Send + Debug + 'static,
{
    let query = shape_mongo_filter(
        query,
        access.map(|e| {
            let Extension(e) = e;
            e
        }),
        Some(headers),
    );

    let store = T::get_store(state.app_stores.clone());

    let filter = query.filter.clone();
    let total = async {
        if count {
            store.count(filter, None).await
        } else {
            Ok(0)
        }
    };

    let find = store.get_many(
        Some(query.filter),
        None,
        None,
        Some(query.limit),
        Some(query.skip),
    );

    let res = match try_join!(find, total) {
        Ok((rows, total)) => ReadResponse {
            rows: rows.into_iter().map(T::public).collect(),
            skip: query.skip,
            limit: query.limit,
            total,
        },
        Err(e) => {
            error!("Error reading from store: {e}");
            return Err(e);
        }
    };

    Ok(Json(ServerResponse::new("read", res)))
}
