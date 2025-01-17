use anyhow::Result;
use dotenvy::dotenv;
use envconfig::Envconfig;
use integrationos_database::algebra::{
    init::{DatabaseInitializer, Initializer},
    on_error_callback,
};
use integrationos_domain::{
    database::DatabasePodConfig,
    telemetry::{get_subscriber, init_subscriber},
};
use tracing::info;

fn main() -> Result<()> {
    dotenv().ok();
    let config = DatabasePodConfig::init_from_env()?;

    let subscriber = get_subscriber("storage".into(), "info".into(), std::io::stdout, None);
    init_subscriber(subscriber);

    info!("Starting Storage API with config:\n{config}");

    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(config.worker_threads.unwrap_or(num_cpus::get()))
        .enable_all()
        .build()?
        .block_on(async move {
            let server = DatabaseInitializer::init(&config).await?;

            if let Err(e) = server.run().await {
                on_error_callback(&e, &config, None).await?;
                return Err(e);
            }

            Ok(())
        })
}
