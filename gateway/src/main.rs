use anyhow::Result;
use dotenvy::dotenv;
use envconfig::Envconfig;
use gateway::finalizer::Finalizer;
use gateway::{config::Config, server::Server};
use integrationos_domain::common::encrypted_data::PASSWORD_LENGTH;
use tracing::info;
use tracing::metadata::LevelFilter;
use tracing_subscriber::EnvFilter;

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<()> {
    dotenv().ok();

    let filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::DEBUG.into())
        .from_env_lossy();
    tracing_subscriber::fmt().with_env_filter(filter).init();

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
