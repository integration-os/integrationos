use crate::{router, service::storage::Storage};
use anyhow::Result as AnyhowResult;
use axum::Router;
use integrationos_domain::database::DatabaseConnectionConfig;
use std::sync::Arc;
use tokio::net::TcpListener;

#[derive(Clone)]
pub struct AppState {
    pub config: DatabaseConnectionConfig,
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

        tracing::info!("Database server listening on {}", self.state.config.address);

        let tcp_listener = TcpListener::bind(&self.state.config.address).await?;

        // Probing the storage to ensure it is up and running before starting the server
        self.state.storage.probe().await.inspect_err(|e| {
            tracing::error!("Could not fetch common model: {e}");
        })?;

        axum::serve(tcp_listener, app.into_make_service())
            .await
            .map_err(|e| anyhow::anyhow!("Server error: {}", e))
    }
}
