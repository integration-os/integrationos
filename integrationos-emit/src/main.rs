use anyhow::Result;
use dotenvy::dotenv;
use envconfig::Envconfig;
use integrationos_domain::telemetry::{get_subscriber, init_subscriber};
use integrationos_emit::{domain::config::EmitterConfig, server::Server};
use std::time::Duration;
use tokio_graceful_shutdown::{SubsystemHandle, Toplevel};

fn main() -> Result<()> {
    dotenv().ok();

    let config = EmitterConfig::init_from_env()?;
    let shutdown_timeout_millis = config.shutdown_timeout_millis;

    let subscriber = get_subscriber("emitter".into(), "info".into(), std::io::stdout, None);
    init_subscriber(subscriber);

    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(config.worker_threads.unwrap_or(num_cpus::get()))
        .enable_all()
        .build()?
        .block_on(async move {
            Toplevel::new(|subsys: SubsystemHandle| async move {
                let server = Server::init(config.clone())
                    .await
                    .expect("Failed to initialize server");

                Server::subsystem(server, &config, subsys).await;
            })
            .catch_signals()
            .handle_shutdown_requests(Duration::from_millis(shutdown_timeout_millis))
            .await
            .map_err(Into::into)
        })
}
