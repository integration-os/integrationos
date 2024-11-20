use anyhow::Result;
use dotenvy::dotenv;
use envconfig::Envconfig;
use integrationos_domain::telemetry::{get_subscriber, init_subscriber};
use integrationos_emit::{domain::config::EmitterConfig, server::Server, stream::EventStreamTopic};
use std::time::Duration;
use tokio_graceful_shutdown::{SubsystemBuilder, SubsystemHandle, Toplevel};

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();

    let config = EmitterConfig::init_from_env()?;
    let shutdown_timeout_millis = config.shutdown_timeout_millis;

    let subscriber = get_subscriber("emitter".into(), "info".into(), std::io::stdout, None);
    init_subscriber(subscriber);

    // tokio::runtime::Builder::new_multi_thread()
    //     .worker_threads(config.worker_threads.unwrap_or(num_cpus::get()))
    //     .enable_all()
    //     .build()?
    //     .block_on(async move {
    //     })

    Toplevel::new(|subsys: SubsystemHandle| async move {
        let server = Server::init(config.clone())
            .await
            .expect("Failed to initialize server");

        let state = server.state.clone();
        let stream = server.state.event_stream.clone();
        let scheduler = server.scheduler.clone();
        let pusher = server.pusher.clone();

        subsys.start(SubsystemBuilder::new(
            EventStreamTopic::Dlq.as_ref(),
            |h| async move { stream.consume(EventStreamTopic::Dlq, h, &state).await },
        ));

        let state = server.state.clone();
        let stream = server.state.event_stream.clone();
        subsys.start(SubsystemBuilder::new(
            EventStreamTopic::Target.as_ref(),
            |s| async move { stream.consume(EventStreamTopic::Target, s, &state).await },
        ));
        //
        let config = server.state.config.clone();
        subsys.start(SubsystemBuilder::new("PusherSubsystem", |s| async move {
            pusher.start(&config, s).await
        }));

        subsys.start(SubsystemBuilder::new(
            "SchedulerSubsystem",
            |s| async move { scheduler.start(s).await },
        ));

        subsys.start(SubsystemBuilder::new("ServerSubsystem", |s| async move {
            server.run(s).await
        }));
    })
    .catch_signals()
    .handle_shutdown_requests(Duration::from_millis(shutdown_timeout_millis))
    .await
    .map_err(Into::into)
}
