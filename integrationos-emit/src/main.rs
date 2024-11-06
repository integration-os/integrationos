use anyhow::Result;
use dotenvy::dotenv;
use envconfig::Envconfig;
use integrationos_domain::telemetry::{get_subscriber, init_subscriber};
use integrationos_emit::{domain::config::EmitterConfig, server::Server};
use tracing::info;

fn main() -> Result<()> {
    dotenv().ok();
    let config = EmitterConfig::init_from_env()?;

    let subscriber = get_subscriber("emitter".into(), "info".into(), std::io::stdout, None);
    init_subscriber(subscriber);

    info!("Starting Emitter API with config:\n{config}");

    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(config.worker_threads.unwrap_or(num_cpus::get()))
        .enable_all()
        .build()?
        .block_on(async move {
            let server: Server = Server::init(config).await?;

            info!("Server started");

            server.run().await
        })
}
