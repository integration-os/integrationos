use crate::{
    domain::{config::EmitterConfig, event::Event, idempotency::Idempotency},
    router,
    stream::{
        fluvio_driver::{FluvioDriverImpl, FluvioDriverLogger},
        EventStreamExt, EventStreamProvider,
    },
};
use anyhow::{Context, Result as AnyhowResult};
use axum::Router;
use futures::Future;
use integrationos_domain::{MongoStore, Store};
use mongodb::{results::CollectionType, Client};
use reqwest_middleware::{reqwest, ClientBuilder, ClientWithMiddleware};
use reqwest_retry::{policies::ExponentialBackoff, RetryTransientMiddleware};
use reqwest_tracing::TracingMiddleware;
use std::{sync::Arc, time::Duration};
use tokio::net::TcpListener;
use tokio_util::sync::CancellationToken;

#[derive(Clone)]
pub struct AppStores {
    pub events: MongoStore<Event>,
    pub idempotency: MongoStore<Idempotency>,
}

#[derive(Clone)]
pub struct AppState {
    pub config: EmitterConfig,
    pub app_stores: AppStores,
    pub http_client: ClientWithMiddleware,
    pub stream_client: Arc<dyn EventStreamExt + Sync + Send>,
}

#[derive(Clone)]
pub struct Server {
    pub state: Arc<AppState>,
    pub token: CancellationToken,
}

impl Server {
    pub async fn init(config: EmitterConfig, token: CancellationToken) -> AnyhowResult<Self> {
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
        };

        let stream_client: Arc<dyn EventStreamExt + Sync + Send> =
            match config.event_stream_provider {
                EventStreamProvider::Logger => Arc::new(FluvioDriverLogger),
                EventStreamProvider::Fluvio => Arc::new(FluvioDriverImpl::new(&config).await?),
            };

        // start events consumer
        let cloned_stream = Arc::clone(&stream_client);
        let cloned_token = token.clone();
        tokio::spawn(async move {
            let res = cloned_stream.consume(cloned_token).await;

            if let Err(ref e) = res {
                tracing::info!("Consumer stopped: {:?}", e);
                std::process::exit(1);
            }
        });

        Ok(Self {
            state: Arc::new(AppState {
                config,
                app_stores,
                http_client,
                stream_client,
            }),
            token,
        })
    }

    pub async fn run(&self) -> AnyhowResult<()> {
        let app = router::get_router(&self.state).await;

        let app: Router<()> = app.with_state(self.state.clone());

        tracing::info!("Emitter server listening on {}", self.state.config.address);

        let tcp_listener = TcpListener::bind(&self.state.config.address).await?;

        axum::serve(tcp_listener, app.into_make_service())
            .with_graceful_shutdown(Server::shutdown(self.token.clone()))
            .await
            .map_err(|e| anyhow::anyhow!("Server error: {}", e))
    }

    pub async fn shutdown(token: CancellationToken) {
        let ctrl_c = async {
            tokio::signal::ctrl_c()
                .await
                .expect("failed to install Ctrl+C handler");
        };

        #[cfg(unix)]
        let terminate = async {
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                .expect("failed to install signal handler")
                .recv()
                .await;
        };

        #[cfg(not(unix))]
        let terminate = std::future::pending::<()>();

        tokio::select! {
            _ = ctrl_c => {
                tracing::info!("Ctrl+C received, shutting down");
                token.cancel();
            },
            _ = terminate => {
                tracing::info!("SIGTERM received, shutting down");
                token.cancel();
            },
        }
    }
}
