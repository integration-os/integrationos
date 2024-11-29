use envconfig::Envconfig;
use http::{Method, StatusCode};
use integrationos_database::{
    domain::postgres::PostgresDatabaseConnection, service::init::Initializer,
};
use integrationos_domain::prefix::IdPrefix;
use integrationos_domain::Id;
use integrationos_domain::{database::DatabaseConnectionConfig, IntegrationOSError, InternalError};
use serde::{de::DeserializeOwned, Serialize};
use serde_json::Value;
use std::fmt::Debug;
use std::{collections::HashMap, sync::OnceLock, time::Duration};
use testcontainers_modules::{
    postgres::Postgres,
    testcontainers::{clients::Cli as Docker, Container},
};
use tokio::net::TcpListener;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::EnvFilter;

static DOCKER: OnceLock<Docker> = OnceLock::new();
static POSTGRES: OnceLock<Container<'static, Postgres>> = OnceLock::new();
static TRACING: OnceLock<()> = OnceLock::new();

pub struct TestServer {
    pub port: u16,
    pub client: reqwest::Client,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ApiResponse<T: DeserializeOwned = Value> {
    pub code: StatusCode,
    pub data: T,
}

impl TestServer {
    pub async fn new() -> Result<Self, IntegrationOSError> {
        TRACING.get_or_init(|| {
            let filter = EnvFilter::builder()
                .with_default_directive(LevelFilter::INFO.into())
                .from_env_lossy();

            tracing_subscriber::fmt().with_env_filter(filter).init();
        });
        let docker = DOCKER.get_or_init(Default::default);
        let postgres = POSTGRES.get_or_init(|| docker.run(Postgres::default()));
        let port = postgres.get_host_port_ipv4(5432);

        let server_port = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("Failed to bind to port")
            .local_addr()
            .expect("Failed to get local address")
            .port();

        let config = DatabaseConnectionConfig::init_from_hashmap(&HashMap::from([
            (
                "INTERNAL_SERVER_ADDRESS".to_string(),
                format!("0.0.0.0:{server_port}"),
            ),
            (
                "DATABASE_CONNECTION_TYPE".to_string(),
                "postgresql".to_string(),
            ),
            ("CONNECTION_ID", Id::now(IdPrefix::Connection).to_string())
            ("POSTGRES_USERNAME".to_string(), "postgres".to_string()),
            ("POSTGRES_PASSWORD".to_string(), "postgres".to_string()),
            ("POSTGRES_HOST".to_string(), "localhost".to_string()),
            ("POSTGRES_PORT".to_string(), port.to_string()),
            ("POSTGRES_NAME".to_string(), "postgres".to_string()),
        ]))
        .expect("Failed to initialize storage config");

        let server = PostgresDatabaseConnection::init(&config)
            .await
            .expect("Failed to initialize storage");

        tokio::task::spawn(async move { server.run().await });

        tokio::time::sleep(Duration::from_millis(50)).await;

        let client = reqwest::Client::new();

        Ok(Self {
            port: server_port,
            client,
        })
    }

    pub async fn send_request<T: Serialize, U: DeserializeOwned + Debug>(
        &self,
        path: &str,
        method: Method,
        payload: Option<&T>,
    ) -> Result<ApiResponse<U>, IntegrationOSError> {
        let uri = format!("http://localhost:{}/{path}", self.port);
        let mut req = self.client.request(method, uri);
        if let Some(payload) = payload {
            req = req.json(payload);
        }

        let res = req
            .send()
            .await
            .map_err(|e| InternalError::io_err(&format!("Failed to send request: {}", e), None))?;

        let status = res.status();
        let json = res.json().await;

        Ok(ApiResponse {
            code: status,
            data: json.map_err(|e| {
                InternalError::deserialize_error(
                    &format!("Failed to deserialize response: {}", e),
                    None,
                )
            })?,
        })
    }
}
