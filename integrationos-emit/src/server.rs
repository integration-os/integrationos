use crate::{
    domain::{
        config::EmitterConfig, deduplication::Deduplication, event::EventEntity,
        idempotency::Idempotency,
    },
    router,
    stream::{
        fluvio_driver::FluvioDriverImpl, logger_driver::LoggerDriverImpl, EventStreamExt,
        EventStreamProvider, EventStreamTopic,
    },
};
use anyhow::Result as AnyhowResult;
use axum::Router;
use axum_server::Handle;
use integrationos_domain::{MongoStore, Store};
use mongodb::Client;
use reqwest_middleware::{reqwest, ClientBuilder, ClientWithMiddleware};
use reqwest_retry::{policies::ExponentialBackoff, RetryTransientMiddleware};
use reqwest_tracing::TracingMiddleware;
use std::{sync::Arc, time::Duration};
use tokio::net::TcpListener;
use tokio_util::sync::CancellationToken;

#[derive(Clone)]
pub struct AppStores {
    pub events: MongoStore<EventEntity>,
    pub idempotency: MongoStore<Idempotency>,
    pub deduplication: MongoStore<Deduplication>,
}

#[derive(Clone)]
pub struct AppState {
    pub config: EmitterConfig,
    pub app_stores: AppStores,
    pub http_client: ClientWithMiddleware,
    pub event_stream: Arc<dyn EventStreamExt + Sync + Send>,
}

#[derive(Clone)]
pub struct Server {
    pub state: Arc<AppState>,
    pub handle: Handle,
}

impl Server {
    pub async fn init(config: EmitterConfig) -> AnyhowResult<Self> {
        let client = Client::with_uri_str(&config.db_config.event_db_url).await?;
        let database = client.database(&config.db_config.event_db_name);

        let retry_policy =
            ExponentialBackoff::builder().build_with_max_retries(config.http_client_max_retries);
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(config.http_client_timeout_secs))
            .build()?;
        let http_client = ClientBuilder::new(client)
            .with(RetryTransientMiddleware::new_with_policy(retry_policy))
            .with(TracingMiddleware::default())
            .build();

        let app_stores = AppStores {
            events: MongoStore::new(&database, &Store::PipelineEvents).await?,
            idempotency: MongoStore::new(&database, &Store::Idempotency).await?,
            deduplication: MongoStore::new(&database, &Store::Deduplication).await?,
        };

        let event_stream: Arc<dyn EventStreamExt + Sync + Send> = match config.event_stream_provider
        {
            EventStreamProvider::Logger => Arc::new(LoggerDriverImpl),
            EventStreamProvider::Fluvio => Arc::new(FluvioDriverImpl::new(&config).await?),
        };

        // start events consumer
        let cloned_stream = Arc::clone(&event_stream);
        let token = CancellationToken::new();
        let cloned_token = token.clone();
        let handle = Handle::new();

        ctrlc::try_set_handler(move || {
            tracing::info!("Received Ctrl+C, shutting down...");
            token.cancel();
        })?;

        let cloned_handle = handle.clone();

        let state = Arc::new(AppState {
            config: config.clone(),
            app_stores,
            http_client,
            event_stream,
        });

        let cloned_state = Arc::clone(&state);
        tokio::spawn(async move {
            let res = cloned_stream
                .consume(cloned_token, EventStreamTopic::Target, &state)
                .await;

            if let Err(ref e) = res {
                tracing::info!("Consumer stopped: {:?}", e);
            }

            handle.shutdown();
        });

        Ok(Self {
            state: cloned_state,
            handle: cloned_handle,
        })
    }

    pub async fn run(&self) -> AnyhowResult<()> {
        let app = router::get_router(&self.state).await;

        let app: Router<()> = app.with_state(self.state.clone());

        tracing::info!("Emitter server listening on {}", self.state.config.address);

        let addr = TcpListener::bind(&self.state.config.address)
            .await?
            .local_addr()?;

        axum_server::bind(addr)
            .handle(self.handle.clone())
            .serve(app.into_make_service())
            .await
            .map_err(|e| anyhow::anyhow!("Server error: {}", e))
    }
}
