use std::sync::Arc;

use anyhow::Result;
use api::{config::Config, server::Server};
use dotenvy::dotenv;
use envconfig::Envconfig;
use integrationos_domain::service::secrets_client::SecretsClient;
use tracing::info;
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::EnvFilter;

fn main() -> Result<()> {
    dotenv().ok();

    let filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::INFO.into())
        .from_env_lossy();

    tracing_subscriber::fmt().with_env_filter(filter).init();

    let config = Config::init_from_env()?;

    info!("Starting API with config:\n{config}");

    let secrets_client = Arc::new(SecretsClient::new(&config.secrets_config)?);

    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(config.worker_threads.unwrap_or(num_cpus::get()))
        .enable_all()
        .build()?
        .block_on(async move {
            let server: Server = Server::init(config, secrets_client).await?;

            server.run().await
        })
}
