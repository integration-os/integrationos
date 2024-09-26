use crate::{
    config::StorageConfig,
    storage::{PostgresStorage, Storage, ThreadSafePgResult},
};
use anyhow::Result as AnyhowResult;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState<T>
where
    T: Send + Sync + Sized,
{
    pub config: StorageConfig,
    pub storage: Arc<dyn Storage<Result = T>>,
}

#[derive(Clone)]
pub struct Server<T>
where
    T: Send + Sync + Sized,
{
    pub state: Arc<AppState<T>>,
}

impl Server<ThreadSafePgResult> {
    // Initialize the Server with the Postgres storage backend
    pub async fn init(config: StorageConfig) -> AnyhowResult<Self> {
        let postgres: PostgresStorage = PostgresStorage::new(&config).await?;
        let storage: Arc<dyn Storage<Result = ThreadSafePgResult>> = Arc::new(postgres);

        Ok(Self {
            state: Arc::new(AppState { config, storage }),
        })
    }

    pub async fn run(&self) -> AnyhowResult<()> {
        // let app = router::get_router(&self.state).await;
        //
        // let app: Router<()> = app.with_state(self.state.clone());
        //
        // info!("Api server listening on {}", self.state.config.address);
        //
        // let tcp_listener = TcpListener::bind(&self.state.config.address).await?;
        //
        // axum::serve(tcp_listener, app.into_make_service())
        //     .await
        //     .map_err(|e| anyhow!("Server error: {}", e))
        todo!()
    }
}
