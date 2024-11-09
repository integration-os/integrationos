use anyhow::Result;
use dotenvy::dotenv;
use envconfig::Envconfig;
use integrationos_domain::{
    telemetry::{get_subscriber, init_subscriber},
    Unit,
};
use integrationos_emit::{domain::config::EmitterConfig, server::Server, stream::EventStreamTopic};
use std::time::Duration;
use tokio_graceful_shutdown::{SubsystemBuilder, SubsystemHandle, Toplevel};
use tracing::info;

async fn subsystem(
    server: Server,
    config: &EmitterConfig,
    subsys: SubsystemHandle,
) -> Result<Unit> {
    info!("Starting Emitter API with config:\n{config}");

    let state = server.state.clone();
    let stream = server.state.event_stream.clone();

    subsys.start(SubsystemBuilder::new(
        EventStreamTopic::Dlq.as_ref(),
        |h| async move { stream.consume(EventStreamTopic::Dlq, h, &state).await },
    ));

    let state = server.state.clone();
    let stream = server.state.event_stream.clone();
    subsys.start(SubsystemBuilder::new(
        EventStreamTopic::Target.as_ref(),
        |h| async move {
            stream
                .consume(EventStreamTopic::Target, h, &state)
                .await
        },
    ));

    server.run().await
}

fn main() -> Result<()> {
    dotenv().ok();

    let config = EmitterConfig::init_from_env()?;
    let shutdown_timeout_secs = config.shutdown_timeout_secs;

    let subscriber = get_subscriber("emitter".into(), "info".into(), std::io::stdout, None);
    init_subscriber(subscriber);

    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(config.worker_threads.unwrap_or(num_cpus::get()))
        .enable_all()
        .build()?
        .block_on(async move {
            Toplevel::new(|s| async move {
                let server = Server::init(config.clone())
                    .await
                    .expect("Failed to initialize server");

                s.start(SubsystemBuilder::new("ServerSubsys", |handle| async move {
                    subsystem(server, &config, handle).await
                }));
            })
            .catch_signals()
            .handle_shutdown_requests(Duration::from_millis(shutdown_timeout_secs))
            .await
            .map_err(Into::into)
        })
}
