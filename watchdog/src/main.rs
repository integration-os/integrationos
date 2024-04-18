use anyhow::{Context, Result};
use dotenvy::dotenv;
use envconfig::Envconfig;
use integrationos_domain::{
    cache::CacheConfig,
    client::watchdog_client::WatchdogClient,
    database::DatabaseConfig,
    telemetry::{get_subscriber, init_subscriber},
    watchdog::WatchdogConfig,
};
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();

    let suscriber = get_subscriber("watchdog".into(), "info".into(), std::io::stdout);
    init_subscriber(suscriber);

    let watchdog_config = WatchdogConfig::init_from_env().context("Could not load config")?;
    let cache_config = CacheConfig::init_from_env().context("Could not load config")?;
    let database_config = DatabaseConfig::init_from_env().context("Could not load config")?;

    info!("Starting watchdog with config: {watchdog_config}{cache_config}{database_config}");

    let client = WatchdogClient::new(watchdog_config, cache_config, database_config);

    client.start().await??;
    Ok(())
}
