use anyhow::Result;
use dotenvy::dotenv;
use envconfig::Envconfig;
use integrationos_api::{domain::config::ConnectionsConfig, server::Server};
use integrationos_domain::telemetry::{get_subscriber, init_subscriber};
use tracing::info;

fn main() -> Result<()> {
    dotenv().ok();
    let config = ConnectionsConfig::init_from_env()?;

    let subscriber = get_subscriber("connections-api".into(), "info".into(), std::io::stdout);
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
