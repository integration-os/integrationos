use crate::{
    domain::{
        config::EmitterConfig,
        deduplication::Deduplication,
        event::{EventEntity, ScheduledEvent},
        idempotency::Idempotency,
    },
    router,
    stream::{
        fluvio_driver::FluvioDriverImpl, logger_driver::LoggerDriverImpl,
        scheduler::PublishScheduler, EventStreamExt, EventStreamProvider,
    },
};
use anyhow::Result as AnyhowResult;
use axum::Router;
use integrationos_domain::{MongoStore, Store, Unit};
use mongodb::Client;
use reqwest_middleware::{reqwest, ClientBuilder, ClientWithMiddleware};
use reqwest_retry::{policies::ExponentialBackoff, RetryTransientMiddleware};
use reqwest_tracing::TracingMiddleware;
use std::{sync::Arc, time::Duration};
use tokio::net::TcpListener;

#[derive(Clone)]
pub struct AppStores {
    pub events: MongoStore<EventEntity>,
    pub idempotency: MongoStore<Idempotency>,
    pub deduplication: MongoStore<Deduplication>,
    pub scheduled: MongoStore<ScheduledEvent>,
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
    pub event_stream: Arc<dyn EventStreamExt + Sync + Send>,
    pub scheduler: Arc<PublishScheduler>,
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
            scheduled: MongoStore::new(&database, &Store::ScheduledEvents).await?,
        };

        let event_stream: Arc<dyn EventStreamExt + Sync + Send> = match config.event_stream_provider
        {
            EventStreamProvider::Logger => Arc::new(LoggerDriverImpl),
            EventStreamProvider::Fluvio => Arc::new(FluvioDriverImpl::new(&config).await?),
        };

        let scheduler = Arc::new(PublishScheduler {
            event_stream: Arc::clone(&event_stream),
            scheduled: app_stores.scheduled.clone(),
            max_concurrent_tasks: config.scheduled_max_concurrent_tasks,
            max_chunk_size: config.scheduled_max_chunk_size,
            sleep_duration: config.scheduled_sleep_duration_millis,
        });

        let state = Arc::new(AppState {
            config: config.clone(),
            app_stores,
            http_client,
            event_stream: Arc::clone(&event_stream),
        });

        Ok(Self {
            state,
            event_stream,
            scheduler,
        })
    }

    pub async fn run(&self) -> AnyhowResult<()> {
        let app = router::get_router(&self.state).await;

        let app: Router<Unit> = app.with_state(self.state.clone());

        tracing::info!("Emitter server listening on {}", self.state.config.address);

        let tcp_listener = TcpListener::bind(&self.state.config.address).await?;

        axum::serve(tcp_listener, app)
            .await
            .map_err(|e| anyhow::anyhow!("Server error: {}", e))
    }
}
