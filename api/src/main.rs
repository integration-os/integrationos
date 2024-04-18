use anyhow::Result;
use api::{config::Config, server::Server};
use dotenvy::dotenv;
use envconfig::Envconfig;
use integrationos_domain::client::secrets_client::SecretsClient;
use integrationos_domain::telemetry::{get_subscriber, init_subscriber};
use std::sync::Arc;
use tracing::info;

fn main() -> Result<()> {
    dotenv().ok();
    let config = Config::init_from_env()?;
    let name = if config.is_admin {
        "admin-api"
    } else {
        "event-api"
    };

    let subscriber = get_subscriber(name.into(), "info".into(), std::io::stdout);
    init_subscriber(subscriber);

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
