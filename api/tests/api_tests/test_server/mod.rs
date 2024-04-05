use std::{
    collections::{BTreeMap, HashMap},
    sync::{Arc, OnceLock, RwLock},
    time::Duration,
};

use anyhow::Result;
use api::{
    config::Config,
    endpoints::{
        connection_model_definition::CreateRequest as CreateConnectionModelDefinitionRequest,
        ReadResponse,
    },
    server::Server,
};
use axum::async_trait;
use envconfig::Envconfig;
use fake::{Fake, Faker};
use http::StatusCode;
use http::{header::AUTHORIZATION, Method};
use integrationos_domain::{
    algebra::adapter::StoreAdapter,
    common::{
        access_key_data::AccessKeyData,
        access_key_prefix::AccessKeyPrefix,
        api_model_config::{AuthMethod, SamplesInput, SchemasInput},
        connection_definition::{ConnectionDefinition, ConnectionDefinitionType},
        connection_model_definition::{
            ConnectionModelDefinition, CrudAction, CrudMapping, PlatformInfo,
        },
        environment::Environment,
        event_access::EventAccess,
        event_type::EventType,
        mongo::MongoDbStore,
        AccessKey, Connection, Store,
    },
    create_secret_response::{CreateSecretAuthor, CreateSecretResponse},
    get_secret_request::GetSecretRequest,
    prelude::crypto::Crypto,
    IntegrationOSError,
};
use mockito::{Matcher, Server as MockServer, ServerGuard};
use mongodb::Client;
use rand::Rng;
use serde::{de::DeserializeOwned, Serialize};
use serde_json::Value;
use serde_json::{from_value, to_value};
use testcontainers_modules::{
    mongo::Mongo,
    redis::Redis,
    testcontainers::{clients::Cli as Docker, Container},
};
use tokio::net::TcpListener;
use tracing_subscriber::{filter::LevelFilter, EnvFilter};
use uuid::Uuid;

use api::endpoints::{
    connection::CreateConnectionPayload,
    connection_definition::CreateRequest as CreateConnectionDefinitionRequest,
};

pub mod test_core;
#[cfg(test)]
pub mod test_gateway;

#[allow(dead_code)]
pub const AUTH_PATHS: &[&str] = &[
    "pipelines",
    "events",
    "transactions",
    "connections",
    "event-access",
];

#[allow(dead_code)]
pub const PUBLIC_PATHS: &[&str] = &["connection-definitions", "openapi"];

static TRACING: OnceLock<()> = OnceLock::new();

pub(crate) static DOCKER: OnceLock<Docker> = OnceLock::new();
static MONGO: OnceLock<Container<'static, Mongo>> = OnceLock::new();
static REDIS: OnceLock<Container<'static, Redis>> = OnceLock::new();

pub struct TestServer {
    port: u16,
    pub config: Config,
    pub live_key: String,
    pub live_access_key: AccessKey,
    pub test_key: String,
    pub test_access_key: AccessKey,
    client: reqwest::Client,
    pub mock_server: ServerGuard,
    pub secrets_client: Arc<MockSecretsClient>,
}

#[derive(Debug, Clone, Default)]
pub struct MockSecretsClient {
    secrets: Arc<RwLock<HashMap<GetSecretRequest, Value>>>,
}

#[async_trait]
impl Crypto for MockSecretsClient {
    async fn encrypt(
        &self,
        key: String,
        value: &serde_json::Value,
    ) -> std::result::Result<CreateSecretResponse, IntegrationOSError> {
        let mut secrets = self.secrets.write().unwrap();
        let id: String = Faker.fake();
        let req = GetSecretRequest {
            id,
            buildable_id: key.clone(),
        };
        secrets.insert(req.clone(), value.clone());

        Ok(CreateSecretResponse {
            id: req.id,
            buildable_id: key,
            created_at: 0f64,
            author: CreateSecretAuthor { id: Faker.fake() },
            encrypted_secret: Faker.fake(),
        })
    }
    async fn decrypt(
        &self,
        secret: &GetSecretRequest,
    ) -> std::result::Result<Value, IntegrationOSError> {
        let secrets = self.secrets.read().unwrap();
        let res = secrets.get(secret).unwrap();
        Ok(res.clone())
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ApiResponse<T: DeserializeOwned = Value> {
    pub code: StatusCode,
    pub data: T,
}

impl TestServer {
    pub async fn new(is_admin: bool, dn_name: Option<String>) -> Self {
        // init tracing once
        TRACING.get_or_init(|| {
            let filter = EnvFilter::builder()
                .with_default_directive(LevelFilter::INFO.into())
                .from_env_lossy();

            tracing_subscriber::fmt().with_env_filter(filter).init();
        });

        // Get db connection
        let docker = DOCKER.get_or_init(Default::default);
        let redis = REDIS.get_or_init(|| docker.run(Redis));
        let mongo = MONGO.get_or_init(|| docker.run(Mongo));
        let host_port = mongo.get_host_port_ipv4(27017);
        let db = format!("mongodb://127.0.0.1:{host_port}/?directConnection=true");

        let host_port = redis.get_host_port_ipv4(6379);
        let redis = format!("redis://127.0.0.1:{host_port}");

        // Get available port for server to listen
        let port = TcpListener::bind("127.0.0.1:0")
            .await
            .unwrap()
            .local_addr()
            .unwrap()
            .port();

        // Random database name
        let db_name = dn_name.unwrap_or_else(|| Uuid::new_v4().to_string());

        let config = Config::init_from_hashmap(&HashMap::from([
            ("CONTROL_DATABASE_URL".to_string(), db.clone()),
            ("CONTROL_DATABASE_NAME".to_string(), db_name.clone()),
            ("CONTEXT_DATABASE_URL".to_string(), db.clone()),
            ("CONTEXT_DATABASE_NAME".to_string(), db_name.clone()),
            ("EVENT_DATABASE_URL".to_string(), db.clone()),
            ("EVENT_DATABASE_NAME".to_string(), db_name.clone()),
            (
                "INTERNAL_SERVER_ADDRESS".to_string(),
                format!("0.0.0.0:{port}"),
            ),
            ("IS_ADMIN".to_string(), is_admin.to_string()),
            ("CLAUDE_API_KEY".to_string(), "".to_string()),
            ("OPENAI_API_KEY".to_string(), "".to_string()),
            ("MOCK_LLM".to_string(), "true".to_string()),
            ("CACHE_SIZE".to_string(), "0".to_string()),
            ("REDIS_URL".to_string(), redis),
        ]))
        .unwrap();

        let secrets_client = Arc::new(MockSecretsClient::default());

        let data: AccessKeyData = Faker.fake();
        let ownership_id = data.id.clone();
        let prefix = AccessKeyPrefix {
            environment: Environment::Live,
            event_type: EventType::SecretKey,
            version: 1,
        };
        let live_access_key = AccessKey {
            prefix,
            data: data.clone(),
        };
        let iv = rand::thread_rng().gen::<[u8; 16]>();
        let live_encrypted_key = live_access_key
            .encode(
                &config.event_access_password.as_bytes().try_into().unwrap(),
                &iv,
            )
            .unwrap();

        let prefix = AccessKeyPrefix {
            environment: Environment::Test,
            event_type: EventType::SecretKey,
            version: 1,
        };
        let test_access_key = AccessKey { prefix, data };
        let test_encrypted_key = test_access_key
            .encode(
                &config.event_access_password.as_bytes().try_into().unwrap(),
                &iv,
            )
            .unwrap();

        // Create live and test keys
        let mut live: EventAccess = Faker.fake();
        live.throughput = 500;
        live.ownership.id = ownership_id.clone().into();
        live.environment = Environment::Live;
        live.record_metadata = Default::default();
        live.access_key = live_encrypted_key.to_string();

        let mut test: EventAccess = Faker.fake();
        test.throughput = 500;
        test.ownership.id = ownership_id.into();
        test.environment = Environment::Test;
        test.record_metadata = Default::default();
        test.access_key = test_encrypted_key.to_string();

        let db = Client::with_uri_str(&db).await.unwrap().database(&db_name);

        let store: MongoDbStore<EventAccess> = MongoDbStore::new_with_db(db, Store::EventAccess)
            .await
            .unwrap();

        store
            .create_many(&[live.clone(), test.clone()])
            .await
            .unwrap();

        let server = Server::init(config.clone(), secrets_client.clone())
            .await
            .unwrap();

        tokio::task::spawn(async move { server.run().await });

        tokio::time::sleep(Duration::from_millis(50)).await;

        Self {
            port,
            config,
            test_key: test.access_key,
            test_access_key,
            live_key: live.access_key,
            live_access_key,
            client: reqwest::Client::new(),
            mock_server: MockServer::new_async().await,
            secrets_client,
        }
    }

    pub async fn send_request<T: Serialize, U: DeserializeOwned>(
        &self,
        path: &str,
        method: http::Method,
        key: Option<&str>,
        payload: Option<&T>,
    ) -> Result<ApiResponse<U>> {
        self.send_request_with_headers(path, method, key, payload, None)
            .await
    }

    pub async fn send_request_with_headers<T: Serialize, U: DeserializeOwned>(
        &self,
        path: &str,
        method: http::Method,
        key: Option<&str>,
        payload: Option<&T>,
        headers: Option<BTreeMap<String, String>>,
    ) -> Result<ApiResponse<U>> {
        let mut req = self
            .client
            .request(method, format!("http://localhost:{}/{path}", self.port));
        if let Some(key) = key {
            req = req.header(&self.config.headers.auth_header, key);
        }
        if let Some(payload) = payload {
            req = req.json(payload);
        }
        if let Some(headers) = headers {
            for (k, v) in headers {
                req = req.header(k, v);
            }
        }

        let res = req.send().await?;

        Ok(ApiResponse {
            code: res.status(),
            data: res.json().await?,
        })
    }

    #[allow(dead_code)]
    pub async fn create_connection(
        &mut self,
        env: Environment,
    ) -> (Connection, ConnectionModelDefinition) {
        let (key, access_key) = match env {
            Environment::Live => (self.live_key.as_ref(), &self.live_access_key),
            Environment::Development => (self.live_key.as_ref(), &self.test_access_key),
            Environment::Test => (self.test_key.as_ref(), &self.test_access_key),
            Environment::Production => (self.live_key.as_ref(), &self.live_access_key),
        };

        let admin_server =
            TestServer::new(true, Some(self.config.db_config.control_db_name.clone())).await;

        let bearer_key: String = Faker.fake();
        let template: String = Faker.fake();
        let handlebar_template = format!("{{{{{template}}}}}");

        let mut connection_def: CreateConnectionDefinitionRequest = Faker.fake();
        connection_def.r#type = ConnectionDefinitionType::Api;
        let mut test_connection: CreateConnectionModelDefinitionRequest = Faker.fake();
        test_connection.base_url = self.mock_server.url();
        test_connection.auth_method = AuthMethod::BearerToken {
            value: handlebar_template.clone(),
        };
        test_connection.http_method = Method::GET;

        let res = admin_server
            .send_request::<CreateConnectionModelDefinitionRequest, ConnectionModelDefinition>(
                "v1/connection-model-definitions",
                http::Method::POST,
                None,
                Some(&test_connection),
            )
            .await
            .unwrap();

        let mut test_connection = res.data;

        let api_config = match test_connection.platform_info {
            PlatformInfo::Api(ref mut api_config_data) => api_config_data.clone(),
            _ => panic!(),
        };

        let mut mock = self
            .mock_server
            .mock(
                test_connection.action.as_str(),
                format!("/{}", api_config.path).as_str(),
            )
            .match_header(
                AUTHORIZATION.as_str(),
                format!("Bearer {bearer_key}").as_str(),
            );
        if let Some(ref headers) = api_config.headers {
            for k in headers.keys() {
                let val: Vec<Matcher> = headers
                    .get_all(k)
                    .into_iter()
                    .map(|v| Matcher::from(v.to_str().unwrap()))
                    .collect();
                if val.len() == 1 {
                    mock = mock.match_header(k.as_str(), Matcher::AllOf(val));
                }
            }
        }
        if let Some(ref query_params) = api_config.query_params {
            let params = query_params
                .iter()
                .map(|(k, v)| Matcher::UrlEncoded(k.into(), v.into()))
                .collect();

            mock = mock.match_query(Matcher::AllOf(params));
        }
        mock = mock
            .expect(1)
            .with_status(200)
            .with_body("\"Charges listed\"")
            .create_async()
            .await;

        connection_def.test_connection = Some(test_connection.id);

        let payload = to_value(&connection_def).unwrap();

        let res = admin_server
            .send_request::<Value, Value>(
                "v1/connection-definitions",
                http::Method::POST,
                None,
                Some(&payload),
            )
            .await
            .unwrap();

        assert!(res.code.is_success());

        let connection_def = from_value::<ConnectionDefinition>(res.data).unwrap();

        let res = self
            .send_request::<Value, Value>(
                &format!("v1/public/connection-definitions?_id={}", connection_def.id),
                http::Method::GET,
                Some(key),
                None,
            )
            .await
            .unwrap();

        assert!(res.code.is_success());

        let res = from_value::<ReadResponse<ConnectionDefinition>>(res.data).unwrap();

        assert_eq!(res.rows.len(), 1);

        let payload = CreateConnectionPayload {
            connection_definition_id: connection_def.id,
            name: Faker.fake(),
            group: access_key.data.group.clone(),
            auth_form_data: HashMap::from([(template, bearer_key.to_string())]),
            active: true,
        };

        let res = self
            .send_request::<CreateConnectionPayload, Connection>(
                "v1/connections",
                http::Method::POST,
                Some(key),
                Some(&payload),
            )
            .await
            .unwrap();

        mock.assert_async().await;
        assert!(res.code.is_success());

        let connection = res.data;

        assert_eq!(connection.platform.to_string(), connection_def.platform);
        assert!(!connection.secrets_service_id.is_empty());

        let model_def = CreateConnectionModelDefinitionRequest {
            connection_platform: connection_def.platform,
            connection_definition_id: connection_def.id,
            platform_version: connection_def.platform_version,
            title: connection_def.name.clone(),
            name: connection_def.name.clone(),
            model_name: connection_def.name.clone(),
            base_url: api_config.base_url,
            path: api_config.path,
            auth_method: api_config.auth_method,
            http_method: test_connection.action,
            action_name: Faker.fake::<CrudAction>(),
            headers: api_config.headers,
            query_params: api_config.query_params,
            extractor_config: test_connection.extractor_config,
            version: test_connection.record_metadata.version,
            schemas: SchemasInput {
                headers: None,
                query_params: None,
                path_params: None,
                body: None,
            },
            samples: SamplesInput {
                headers: None,
                query_params: None,
                path_params: None,
                body: None,
            },
            responses: vec![],
            paths: None,
            is_default_crud_mapping: None,
            mapping: Some(CrudMapping {
                action: CrudAction::GetMany,
                common_model_name: connection_def.name,
                from_common_model: Some(
                    "function mapCrudRequest(data) { return data; }".to_string(),
                ),
                to_common_model: Some("function mapCrudRequest(data) { return data; }".to_string()),
            }),
        };

        let res = admin_server
            .send_request::<CreateConnectionModelDefinitionRequest, ConnectionModelDefinition>(
                "v1/connection-model-definitions",
                http::Method::POST,
                None,
                Some(&model_def),
            )
            .await
            .unwrap();
        assert!(res.code.is_success());

        let conn_model_def = res.data;

        (connection, conn_model_def)
    }
}
