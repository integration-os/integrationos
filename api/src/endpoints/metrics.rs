use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    routing::get,
    Json, Router,
};
use bson::Document;
use integrationos_domain::Store;
use serde::{Deserialize, Serialize};
use tracing::error;

use crate::{
    internal_server_error,
    metrics::{DAILY_KEY, MONTHLY_KEY, PLATFORMS_KEY, TOTAL_KEY},
    not_found,
    server::AppState,
};

use super::ApiResult;

pub fn get_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(get_metrics))
        .route("/:client_id", get(get_metrics))
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Granularity {
    Day(String),
    Month(String),
    #[default]
    Total,
}

#[derive(Debug, Clone, Copy, strum::Display, Deserialize)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum MetricType {
    Passthrough,
    Unified,
    RateLimited,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct QueryParams {
    #[serde(default, rename = "apiType")]
    metric_type: Option<MetricType>,
    #[serde(default)]
    platform: Option<String>,
    #[serde(flatten)]
    granularity: Option<Granularity>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MetricResponse {
    pub count: i32,
}

pub async fn get_metrics(
    state: State<Arc<AppState>>,
    path: Option<Path<String>>,
    query_params: Option<Query<QueryParams>>,
) -> ApiResult<MetricResponse> {
    let coll = state
        .app_stores
        .db
        .collection::<Document>(&Store::Metrics.to_string());

    let client_id = path
        .and_then(|p| if p.0.is_empty() { None } else { Some(p) })
        .map(|p| p.0)
        .unwrap_or(state.config.metric_system_id.clone());

    let doc = match coll
        .find_one(bson::doc! { "clientId": client_id }, None)
        .await
    {
        Ok(Some(doc)) => doc,
        Ok(None) => return Err(not_found!("Client")),
        Err(e) => {
            error!("Could not fetch metric: {e}");
            return Err(internal_server_error!());
        }
    };

    let query_params = query_params.unwrap_or_default();

    let metric_type = query_params.metric_type.unwrap_or(MetricType::Unified);
    println!(
        "metric type: {:?} - {}",
        query_params.metric_type, metric_type
    );
    let Ok(doc) = doc.get_document(metric_type.to_string()) else {
        return Ok(Json(MetricResponse { count: 0 }));
    };

    let doc = if let Some(platform) = &query_params.platform {
        let Ok(doc) = doc
            .get_document(PLATFORMS_KEY)
            .and_then(|d| d.get_document(platform))
        else {
            return Ok(Json(MetricResponse { count: 0 }));
        };
        doc
    } else {
        doc
    };

    let result = match query_params
        .granularity
        .as_ref()
        .unwrap_or(&Granularity::Total)
    {
        Granularity::Day(day) => doc.get_document(DAILY_KEY).and_then(|d| d.get_i32(day)),
        Granularity::Month(month) => doc.get_document(MONTHLY_KEY).and_then(|d| d.get_i32(month)),
        Granularity::Total => doc.get_i32(TOTAL_KEY),
    };

    Ok(Json(MetricResponse {
        count: result.unwrap_or_default(),
    }))
}
