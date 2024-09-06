use anyhow::{Context, Result};
use dotenvy::dotenv;
use envconfig::Envconfig;
use integrationos_domain::{
    create_secret_response::Secret,
    secrets::SecretServiceProvider,
    telemetry::{get_subscriber, init_subscriber},
    GoogleCryptoKms, GoogleKms, IOSKms, MongoStore, SecretExt, Store,
};
use integrationos_event::{
    config::EventCoreConfig,
    dispatcher::Dispatcher,
    event_handler::EventHandler,
    metrics::{CONCURRENT_EVENTS_GAUGE, CONCURRENT_EVENTS_PERCENTAGE_GAUGE},
    mongo_context_store::MongoContextStore,
    mongo_control_data_store::MongoControlDataStore,
};
use metrics_exporter_prometheus::PrometheusBuilder;
use mongodb::Client;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_condvar::Condvar;
use tracing::{error, info, warn};

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();

    let suscriber = get_subscriber("integrationos-event".into(), "info".into(), std::io::stdout);
    init_subscriber(suscriber);

    let config = EventCoreConfig::init_from_env()?;

    info!("Starting integrationos-event with config: {config}");

    PrometheusBuilder::new()
        .install()
        .with_context(|| "failed to install prometheus server")?;

    metrics::describe_gauge!(
        CONCURRENT_EVENTS_GAUGE,
        "number of events currently being concurrently executed by this worker"
    );

    metrics::describe_gauge!(
        CONCURRENT_EVENTS_PERCENTAGE_GAUGE,
        "percentage of total capacity of events currently being concurrently executed by this worker"
    );

    let client = Client::with_uri_str(&config.db_config.event_db_url).await?;
    let database = client.database(&config.db_config.event_db_name);
    let secrets_store = MongoStore::<Secret>::new(&database, &Store::Secrets).await?;

    let secrets_client: Arc<dyn SecretExt + Sync + Send> = match config.secrets_config.provider {
        SecretServiceProvider::GoogleKms => {
            Arc::new(GoogleKms::new(&config.secrets_config, secrets_store).await?)
        }
        SecretServiceProvider::IOSKms => {
            Arc::new(IOSKms::new(&config.secrets_config, secrets_store).await?)
        }
    };

    let control_store = Arc::new(
        MongoControlDataStore::new(&config, secrets_client)
            .await
            .with_context(|| "Could not connect to mongo db")?,
    );

    let context_store = Arc::new(
        MongoContextStore::new(&config)
            .await
            .with_context(|| "Could not connect to context store db")?,
    );

    let dispatcher = Dispatcher {
        context_store: context_store.clone(),
        event_store: control_store.clone(),
        control_data_store: control_store.clone(),
    };

    let event_handler =
        EventHandler::new(config.cache, control_store.clone(), context_store).await?;

    info!("Listening for events on redis...");
    let sync_pair = Arc::new((Mutex::new(0u64), Condvar::new()));

    loop {
        let event_with_context = event_handler.pop_event().await?;
        increment_task_count(sync_pair.clone(), config.db_connection_count).await;
        let sync_pair_clone = sync_pair.clone();
        let control_store = control_store.clone();
        let dispatcher = dispatcher.clone();
        let event_handler = event_handler.clone();
        tokio::spawn(async move {
            match event_handler
                .increment_throughput_count(&event_with_context.event)
                .await
            {
                Ok(below_limit) => {
                    if !below_limit {
                        warn!(
                            "Throughput limit hit for {}, sending to back of queue",
                            event_with_context.event.id
                        );
                        if let Err(e) = event_handler.defer_event(event_with_context).await {
                            error!("Could not send event back to redis: {e}");
                        }
                        decrement_task_count(sync_pair_clone, config.db_connection_count).await;
                        return;
                    }
                }
                Err(e) => {
                    error!("Failed to increment throughput count: {e}");
                }
            }

            control_store
                .event_cache
                .insert(
                    event_with_context.event.id,
                    event_with_context.event.clone(),
                )
                .await;

            if let Err(e) = dispatcher.process_context(event_with_context.context).await {
                error!("Could not process event: {e}");
            }
            decrement_task_count(sync_pair_clone, config.db_connection_count).await;
        });
        let mut task_count: tokio::sync::MutexGuard<'_, u64> = sync_pair.0.lock().await;
        while *task_count >= config.db_connection_count {
            task_count = sync_pair.1.wait(task_count).await;
        }
    }
}

async fn increment_task_count(sync_pair: Arc<(Mutex<u64>, Condvar)>, connection_count: u64) {
    let mut task_count = sync_pair.0.lock().await;
    *task_count += 1;
    metrics::gauge!(CONCURRENT_EVENTS_GAUGE, *task_count as f64);
    metrics::gauge!(
        CONCURRENT_EVENTS_PERCENTAGE_GAUGE,
        *task_count as f64 / connection_count as f64
    );
    sync_pair.1.notify_one();
}

async fn decrement_task_count(sync_pair: Arc<(Mutex<u64>, Condvar)>, connection_count: u64) {
    let mut task_count = sync_pair.0.lock().await;
    *task_count -= 1;
    metrics::gauge!(CONCURRENT_EVENTS_GAUGE, *task_count as f64);
    metrics::gauge!(
        CONCURRENT_EVENTS_PERCENTAGE_GAUGE,
        *task_count as f64 / connection_count as f64
    );
    sync_pair.1.notify_one();
}
