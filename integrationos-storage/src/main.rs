use anyhow::Result;
use dotenvy::dotenv;
use envconfig::Envconfig;
use integrationos_domain::telemetry::{get_subscriber, init_subscriber};
use integrationos_storage::{
    domain::{
        config::{StorageConfig, StorageConfigType},
        postgres::PostgresStorage,
    },
    service::init::Initializer,
};
use tracing::info;

fn main() -> Result<()> {
    dotenv().ok();
    let config = StorageConfig::init_from_env()?;

    let subscriber = get_subscriber("storage".into(), "info".into(), std::io::stdout);
    init_subscriber(subscriber);

    info!("Starting Storage API with config:\n{config}");

    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(config.worker_threads.unwrap_or(num_cpus::get()))
        .enable_all()
        .build()?
        .block_on(async move {
            match config.storage_config_type {
                StorageConfigType::Postgres => PostgresStorage::init(&config).await?.run().await,
            }
        })
}
