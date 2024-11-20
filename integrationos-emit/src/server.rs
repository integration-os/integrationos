use crate::{
    domain::{
        config::EmitterConfig,
        deduplication::Deduplication,
        event::{EventEntity, ScheduledEvent},
        idempotency::Idempotency,
    },
    router,
    stream::{
        fluvio_driver::FluvioDriverImpl, logger_driver::LoggerDriverImpl, pusher::EventPusher,
        scheduler::PublishScheduler, EventStreamExt, EventStreamProvider, EventStreamTopic,
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
use tokio::{net::TcpListener, signal};
use tokio_graceful_shutdown::{SubsystemBuilder, SubsystemHandle};

#[derive(Clone)]
pub struct AppStores {
    pub events: MongoStore<EventEntity>,
    pub idempotency: MongoStore<Idempotency>,
    pub deduplication: MongoStore<Deduplication>,
    pub scheduled: MongoStore<ScheduledEvent>,
    pub client: Client,
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
    pub pusher: Arc<EventPusher>,
}

impl Server {
    pub async fn init(config: EmitterConfig) -> AnyhowResult<Self> {
        let client = Client::with_uri_str(&config.db_config.event_db_url).await?;
        let database = client.database(&config.db_config.event_db_name);

        let app_stores = AppStores {
            events: MongoStore::new(&database, &Store::PipelineEvents).await?,
            idempotency: MongoStore::new(&database, &Store::Idempotency).await?,
            deduplication: MongoStore::new(&database, &Store::Deduplication).await?,
            scheduled: MongoStore::new(&database, &Store::ScheduledEvents).await?,
            client,
        };

        let retry_policy =
            ExponentialBackoff::builder().build_with_max_retries(config.http_client_max_retries);
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(config.http_client_timeout_secs))
            .build()?;
        let http_client = ClientBuilder::new(client)
            .with(RetryTransientMiddleware::new_with_policy(retry_policy))
            .with(TracingMiddleware::default())
            .build();

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

        let pusher = Arc::new(EventPusher {
            event_stream: Arc::clone(&event_stream),
            events: app_stores.events.clone(),
            deduplication: app_stores.deduplication.clone(),
            max_concurrent_tasks: config.pusher_max_concurrent_tasks,
            max_chunk_size: config.pusher_max_chunk_size,
            sleep_duration: config.pusher_sleep_duration_millis,
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
            pusher,
        })
    }

    pub async fn run(&self, subsys: SubsystemHandle) -> AnyhowResult<()> {
        let app = router::get_router(&self.state).await;

        let app: Router<Unit> = app.with_state(self.state.clone());

        tracing::info!("Emitter server listening on {}", self.state.config.address);

        let tcp_listener = TcpListener::bind(&self.state.config.address).await?;

        axum::serve(tcp_listener, app)
            .with_graceful_shutdown(Self::shutdown(subsys))
            .await
            .map_err(|e| anyhow::anyhow!("Server error: {}", e))
    }

    async fn shutdown(subsys: SubsystemHandle) {
        let ctrl_c = async {
            signal::ctrl_c()
                .await
                .expect("failed to install Ctrl+C handler");
        };

        #[cfg(unix)]
        let terminate = async {
            signal::unix::signal(signal::unix::SignalKind::terminate())
                .expect("failed to install signal handler")
                .recv()
                .await;
        };

        #[cfg(not(unix))]
        let terminate = std::future::pending::<()>();

        tokio::select! {
            _ = ctrl_c => {
                subsys.on_shutdown_requested().await;
            },
            _ = terminate => {
                subsys.on_shutdown_requested().await;
            },
        }
        tracing::info!("Starting server shutdown ...");
    }

    pub async fn subsystem(
        server: Server,
        config: &EmitterConfig,
        subsys: SubsystemHandle,
    ) -> Unit {
        tracing::info!("Starting Emitter API with config:\n{config}");

        let state = server.state.clone();
        let stream = server.state.event_stream.clone();
        let scheduler = server.scheduler.clone();
        let pusher = server.pusher.clone();

        subsys.start(SubsystemBuilder::new(
            EventStreamTopic::Dlq.as_ref(),
            |h| async move { stream.consume(EventStreamTopic::Dlq, h, &state).await },
        ));

        let state = server.state.clone();
        let stream = server.state.event_stream.clone();
        subsys.start(SubsystemBuilder::new(
            EventStreamTopic::Target.as_ref(),
            |s| async move { stream.consume(EventStreamTopic::Target, s, &state).await },
        ));
        //
        let config = server.state.config.clone();
        subsys.start(SubsystemBuilder::new("PusherSubsystem", |s| async move {
            pusher.start(&config, s).await
        }));

        subsys.start(SubsystemBuilder::new(
            "SchedulerSubsystem",
            |s| async move { scheduler.start(s).await },
        ));

        subsys.start(SubsystemBuilder::new("ServerSubsystem", |s| async move {
            server.run(s).await
        }));
    }
}
