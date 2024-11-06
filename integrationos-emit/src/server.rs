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
        scheduler::PublishScheduler, EventStreamExt, EventStreamProvider, EventStreamTopic,
    },
};
use anyhow::Result as AnyhowResult;
use axum::Router;
use axum_server::Handle;
use integrationos_domain::{MongoStore, Store, Unit};
use mongodb::Client;
use reqwest_middleware::{reqwest, ClientBuilder, ClientWithMiddleware};
use reqwest_retry::{policies::ExponentialBackoff, RetryTransientMiddleware};
use reqwest_tracing::TracingMiddleware;
use std::{sync::Arc, time::Duration};
use strum::IntoEnumIterator;
use tokio_util::sync::CancellationToken;

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
    pub scheduler: Arc<PublishScheduler>,
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
            scheduled: MongoStore::new(&database, &Store::ScheduledEvents).await?,
        };

        let event_stream: Arc<dyn EventStreamExt + Sync + Send> = match config.event_stream_provider
        {
            EventStreamProvider::Logger => Arc::new(LoggerDriverImpl),
            EventStreamProvider::Fluvio => Arc::new(FluvioDriverImpl::new(&config).await?),
        };

        let token = CancellationToken::new();
        let handle = Handle::new();
        ctrlc::try_set_handler({
            let token = token.clone();
            move || {
                tracing::info!("Received Ctrl+C, shutting down...");
                token.cancel();
            }
        })?;

        let scheduler = Arc::new(PublishScheduler {
            event_stream: Arc::clone(&event_stream),
            scheduled: app_stores.scheduled.clone(),
            max_concurrent_tasks: config.scheduled_max_concurrent_tasks,
            max_chunk_size: config.scheduled_max_chunk_size,
            sleep_duration: config.scheduled_sleep_duration_millis,
        });

        let cloned_scheduler = Arc::clone(&scheduler);
        tokio::spawn(async move {
            cloned_scheduler.start().await;
        });

        let state = Arc::new(AppState {
            config: config.clone(),
            app_stores,
            http_client,
            event_stream: Arc::clone(&event_stream),
            scheduler,
        });

        let is_logger = state.config.event_stream_provider == EventStreamProvider::Logger;

        for topic in EventStreamTopic::iter() {
            let cloned_stream = Arc::clone(&event_stream);
            let cloned_state = Arc::clone(&state);
            let cloned_token = token.clone();
            let cloned_handle = handle.clone();

            tokio::spawn(async move {
                let result = cloned_stream
                    .consume(cloned_token, topic, &cloned_state)
                    .await;

                if let Err(ref e) = result {
                    tracing::info!("{} consumer stopped: {:?}", topic.as_ref(), e);
                }

                if !is_logger {
                    cloned_handle.shutdown();
                }
            });
        }

        Ok(Self { state, handle })
    }

    pub async fn run(&self) -> AnyhowResult<()> {
        let app = router::get_router(&self.state).await;

        let app: Router<Unit> = app.with_state(self.state.clone());

        tracing::info!("Emitter server listening on {}", self.state.config.address);

        axum_server::bind(self.state.config.address)
            .handle(self.handle.clone())
            .serve(app.into_make_service())
            .await
            .map_err(|e| anyhow::anyhow!("Server error: {}", e))?;

        Ok(())
    }
}
