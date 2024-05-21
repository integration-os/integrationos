use anyhow::Result;
use dotenvy::dotenv;
use envconfig::Envconfig;
use integrationos_domain::encrypted_data::PASSWORD_LENGTH;
use integrationos_domain::telemetry::{get_subscriber, init_subscriber};
use integrationos_gateway::finalizer::Finalizer;
use integrationos_gateway::{config::Config, server::Server};
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();

    let suscriber = get_subscriber(
        "integrationos-gateway".into(),
        "info".into(),
        std::io::stdout,
    );
    init_subscriber(suscriber);

    let config = Config::init_from_env()?;
    if config.secret_key.len() != PASSWORD_LENGTH {
        panic!(
            "Secret key must be {PASSWORD_LENGTH} characters long, provided key is {} characters long",
            config.secret_key.len()
        );
    }

    info!("Starting integrationos-gateway with config: {config}");

    let finalizer = Finalizer::new(config.clone()).await?;

    let server = Server::new(config, finalizer);

    server.run().await?;

    Ok(())
}
