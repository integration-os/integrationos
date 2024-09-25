use anyhow::Result;
use dotenvy::dotenv;
use envconfig::Envconfig;
use integrationos_domain::secret::Secret;
use integrationos_domain::secrets::SecretServiceProvider;
use integrationos_domain::telemetry::{get_subscriber, init_subscriber};
use integrationos_domain::{GoogleKms, IOSKms, MongoStore, SecretExt, Store};
use integrationos_storage::{config::StorageConfig, server::Server};
use mongodb::Client;
use std::sync::Arc;
use tracing::info;

fn main() -> Result<()> {
    dotenv().ok();
    let config = StorageConfig::init_from_env()?;

    let subscriber = get_subscriber(
        "single-tenant-storage".into(),
        "info".into(),
        std::io::stdout,
    );
    init_subscriber(subscriber);

    info!("Starting API with config:\n{config}");

    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(config.worker_threads.unwrap_or(num_cpus::get()))
        .enable_all()
        .build()?
        .block_on(async move {
            let server: Server = Server::init(config).await?;

            server.run().await
        })
}
