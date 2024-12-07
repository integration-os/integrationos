use crate::server::AppState;
use axum::{routing::get, Router};
use std::{future::ready, sync::Arc};

pub async fn get_router(state: &Arc<AppState>) -> Router<Arc<AppState>> {
    let metrics_handle = state.metrics.as_ref().1.clone();
    Router::new().route("/metrics", get(move || ready(metrics_handle.render())))
}
