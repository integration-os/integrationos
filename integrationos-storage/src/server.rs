use crate::{domain::config::StorageConfig, router, service::storage::Storage};
use anyhow::Result as AnyhowResult;
use axum::Router;
use std::sync::Arc;
use tokio::net::TcpListener;

#[derive(Clone)]
pub struct AppState {
    pub config: StorageConfig,
    pub storage: Arc<dyn Storage>,
}

#[derive(Clone)]
pub struct Server {
    pub state: Arc<AppState>,
}

impl Server {
    pub async fn run(&self) -> AnyhowResult<()> {
        let app = router::get_router().await;

        let app: Router<()> = app.with_state(self.state.clone());

        tracing::info!("Api server listening on {}", self.state.config.address);

        let tcp_listener = TcpListener::bind(&self.state.config.address).await?;

        axum::serve(tcp_listener, app.into_make_service())
            .await
            .map_err(|e| anyhow::anyhow!("Server error: {}", e))
    }
}
