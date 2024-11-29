use anyhow::Result;
use dotenvy::dotenv;
use envconfig::Envconfig;
use integrationos_database::{
    domain::postgres::PostgresDatabaseConnection, service::init::Initializer,
};
use integrationos_domain::{
    database::{DatabaseConnectionConfig, DatabaseConnectionType},
    telemetry::{get_subscriber, init_subscriber},
};
use tracing::info;

fn main() -> Result<()> {
    dotenv().ok();
    let config = DatabaseConnectionConfig::init_from_env()?;

    let subscriber = get_subscriber("storage".into(), "info".into(), std::io::stdout, None);
    init_subscriber(subscriber);

    info!("Starting Storage API with config:\n{config}");

    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(config.worker_threads.unwrap_or(num_cpus::get()))
        .enable_all()
        .build()?
        .block_on(async move {
            match config.database_connection_type {
                DatabaseConnectionType::PostgreSql => {
                    match PostgresDatabaseConnection::init(&config).await {
                        Ok(server) => {
                            if let Err(e) = server.run().await {
                                PostgresDatabaseConnection::kill(&config).await?;
                                return Err(anyhow::anyhow!("Could not run server: {e}"));
                            }

                            Ok(())
                        }
                        Err(e) => {
                            tracing::error!("Could not initialize storage: {e}");

                            PostgresDatabaseConnection::kill(&config).await?;

                            Err(anyhow::anyhow!("Could not initialize storage: {e}"))
                        }
                    }
                }
            }
        })
}
