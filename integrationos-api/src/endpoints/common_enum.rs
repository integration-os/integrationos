use super::{ApiError, ReadResponse};
use crate::{internal_server_error, server::AppState, util::shape_mongo_filter};
use axum::{
    extract::{Query, State},
    Json,
};
use integrationos_domain::common_model::CommonEnum;
use shape_mongo_filter::DELETED_STR;
use std::{collections::BTreeMap, sync::Arc};
use tokio::try_join;
use tracing::error;

#[tracing::instrument(name = "read::common_enum", skip(query, state))]
pub async fn read(
    query: Option<Query<BTreeMap<String, String>>>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<ReadResponse<CommonEnum>>, ApiError> {
    let mut query = shape_mongo_filter(query, None, None);
    query.filter.remove(DELETED_STR);

    let store = &state.app_stores.common_enum;
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
            return Err(internal_server_error!());
        }
    };

    Ok(Json(res))
}
