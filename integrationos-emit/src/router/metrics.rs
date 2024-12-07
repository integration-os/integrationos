use crate::server::AppState;
use axum::{routing::get, Router};
use std::{future::ready, sync::Arc};

pub async fn get_router(state: &Arc<AppState>) -> Router<Arc<AppState>> {
    let routes = Router::new();

    match state.metrics.as_ref().clone() {
        Some((_, metric_handle)) => {
            routes.route("/metrics", get(move || ready(metric_handle.render())))
        }
        None => routes,
    }
}
