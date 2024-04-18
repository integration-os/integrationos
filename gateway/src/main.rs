use anyhow::Result;
use dotenvy::dotenv;
use envconfig::Envconfig;
use gateway::finalizer::Finalizer;
use gateway::{config::Config, server::Server};
use integrationos_domain::encrypted_data::PASSWORD_LENGTH;
use integrationos_domain::telemetry::{get_subscriber, init_subscriber};
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();

    let suscriber = get_subscriber("gateway".into(), "info".into(), std::io::stdout);
    init_subscriber(suscriber);

    let config = Config::init_from_env()?;
    if config.secret_key.len() != PASSWORD_LENGTH {
        panic!(
            "Secret key must be {PASSWORD_LENGTH} characters long, provided key is {} characters long",
            config.secret_key.len()
        );
    }

    info!("Starting gateway with config: {config}");

    let finalizer = Finalizer::new(config.clone()).await?;

    let server = Server::new(config, finalizer);

    server.run().await?;

    Ok(())
}
