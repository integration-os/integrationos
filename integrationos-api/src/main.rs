use anyhow::Result;
use dotenvy::dotenv;
use envconfig::Envconfig;
use integrationos_api::{config::ConnectionsConfig, server::Server};
use integrationos_domain::create_secret_response::Secret;
use integrationos_domain::secrets::SecretServiceProvider;
use integrationos_domain::telemetry::{get_subscriber, init_subscriber};
use integrationos_domain::{GoogleKms, IOSKms, MongoStore, SecretExt, Store};
use mongodb::Client;
use std::sync::Arc;
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
            let client = Client::with_uri_str(&config.db_config.event_db_url).await?;
            let database = client.database(&config.db_config.event_db_name);
            let secrets_store = MongoStore::<Secret>::new(&database, &Store::Secrets).await?;
            let secrets_client: Arc<dyn SecretExt + Sync + Send> =
                match config.secrets_config.provider {
                    SecretServiceProvider::GoogleKms => {
                        Arc::new(GoogleKms::new(&config.secrets_config, secrets_store).await?)
                    }
                    SecretServiceProvider::IOSKms => {
                        Arc::new(IOSKms::new(&config.secrets_config, secrets_store).await?)
                    }
                };

            let server: Server = Server::init(config, secrets_client).await?;

            server.run().await
        })
}
