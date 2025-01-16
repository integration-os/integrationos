use crate::context::DOCKER;
use anyhow::Result;
use envconfig::Envconfig;
use http::StatusCode;
use integrationos_api::domain::config::ConnectionsConfig as ApiConfig;
use integrationos_domain::event_response::EventResponse;
use integrationos_gateway::{config::Config, finalizer::Finalizer, server::Server};
use serde_json::{json, Value};
use std::collections::HashMap;
use testcontainers_modules::{redis::Redis, testcontainers::Container};
use tokio::net::TcpListener;
use uuid::Uuid;

#[allow(dead_code)]
pub struct TestGateway {
    port: u16,
    pub config: Config,
    client: reqwest::Client,
    _redis: Container<'static, Redis>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ApiResponse {
    pub code: StatusCode,
    pub data: EventResponse,
}

impl TestGateway {
    #[allow(dead_code)]
    pub async fn new(api_config: &ApiConfig) -> Self {
        // Get available port for server to listen
        let port = TcpListener::bind("127.0.0.1:0")
            .await
            .unwrap()
            .local_addr()
            .unwrap()
            .port();

        let docker = DOCKER.get_or_init(Default::default);
        let node = docker.run(Redis);
        let host_port = node.get_host_port_ipv4(6379);
        let redis = format!("redis://127.0.0.1:{host_port}");

        let queue_name = Uuid::new_v4().to_string();

        let mut config = Config::init_from_hashmap(&HashMap::from([
            ("SERVER_ADDRESS".to_string(), format!("0.0.0.0:{port}")),
            ("REDIS_URL".to_string(), redis),
            ("REDIS_QUEUE_NAME".to_string(), queue_name),
        ]))
        .unwrap();

        config.db = api_config.db_config.clone();

        let finalizer = Finalizer::new(config.clone()).await.unwrap();
        let server = Server::new(config.clone(), finalizer);

        tokio::task::spawn(async move { server.run().await });

        Self {
            port,
            config,
            client: reqwest::Client::new(),
            _redis: node,
        }
    }

    #[allow(dead_code)]
    pub async fn emit_event(&self, key: &str, name: &str, payload: &Value) -> Result<ApiResponse> {
        let req = self
            .client
            .post(format!("http://localhost:{}/emit", self.port))
            .header("x-pica-secret", key)
            .json(&json!({"event": name, "payload": payload}));

        let res = req.send().await?;

        Ok(ApiResponse {
            code: res.status(),
            data: res.json().await?,
        })
    }
}
