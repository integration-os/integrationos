use chrono::Utc;
use envconfig::Envconfig;
use fake::{
    faker::{internet::en::FreeEmail, name::en::Name},
    Fake, Faker,
};
use http::Method;
use integrationos_domain::{
    algebra::CryptoExt,
    api_model_config::{ApiModelConfig, AuthMethod, SamplesInput, SchemasInput},
    connection_model_definition::{
        ConnectionModelDefinition, CrudAction, PlatformInfo, TestConnection,
    },
    create_secret_response::Secret,
    destination::Action,
    environment::Environment,
    get_secret_request::GetSecretRequest,
    id::{prefix::IdPrefix, Id},
    ownership::Ownership,
    record_metadata::RecordMetadata,
    settings::Settings,
    Connection, ConnectionType, IntegrationOSError, Pipeline, SecretAuthor, SecretExt,
    SecretVersion, Throughput,
};
use integrationos_event::{
    config::EventCoreConfig, mongo_control_data_store::MongoControlDataStore,
    store::ControlDataStore,
};
use mockito::Server;
use mongodb::Client;
use serde_json::{json, Value};
use std::{collections::HashMap, str::FromStr, sync::Arc};
use testcontainers_modules::{mongo::Mongo, testcontainers::clients::Cli as Docker};
use uuid::Uuid;

pub async fn seed_db(config: &EventCoreConfig, base_url: String) -> Id {
    let client = Client::with_uri_str(&config.db_config.control_db_url)
        .await
        .unwrap();
    let db = client.database(&config.db_config.control_db_name);
    let ts = Utc::now();
    let uuid = Uuid::nil();
    let event_access_id = Id::new_with_uuid(IdPrefix::EventAccess, ts, uuid);

    let stripe_model_config = ConnectionModelDefinition {
        id: Id::from_str("conn_mod_def::AAAAAAAAAAA::AAAAAAAAAAAAAAAAAAAAAA").unwrap(),
        platform_version: "2023-08-16".to_string(),
        connection_platform: "stripe".to_string(),
        connection_definition_id: Id::from_str("conn::AAAAAAAAAAA::AAAAAAAAAAAAAAAAAAAAAA")
            .unwrap(),
        title: "Create Stripe Customers".to_string(),
        name: "customers".to_string(),
        key: "api::stripe::v1::customer::create::create_customer".to_string(),
        model_name: "Customers".to_string(),
        action_name: Faker.fake::<CrudAction>(),
        platform_info: PlatformInfo::Api(ApiModelConfig {
            base_url,
            path: "customers".to_string(),
            auth_method: AuthMethod::BearerToken {
                value: "{{STRIPE_SECRET_KEY}}".to_string(),
            },
            headers: None,
            query_params: None,
            schemas: SchemasInput {
                headers: None,
                query_params: None,
                path_params: None,
                body: None,
            },
            content: None,
            samples: SamplesInput {
                headers: None,
                query_params: None,
                path_params: None,
                body: None,
            },
            responses: vec![],
            paths: None,
        }),
        action: http::Method::POST,
        extractor_config: None,
        test_connection_status: TestConnection::default(),
        record_metadata: Default::default(),
        is_default_crud_mapping: None,
        mapping: None,
        supported: true,
    };

    db.collection("connection-model-definitions")
        .insert_one(
            bson::to_bson_with_options(&stripe_model_config, Default::default())
                .expect("Unable to serialize connection model definition"),
            None,
        )
        .await
        .unwrap();

    let conn = Connection {
        id: Id::new_with_uuid(IdPrefix::Connection, ts, uuid),
        platform_version: "platformVersion".to_string(),
        connection_definition_id: Id::new_with_uuid(IdPrefix::ConnectionDefinition, ts, uuid),
        r#type: ConnectionType::Api {},
        name: "name".to_string(),
        key: "key".into(),
        group: "group".to_string(),
        platform: "platform".to_string().into(),
        environment: Environment::Live,
        secrets_service_id: "secrets_service_id".to_string(),
        event_access_id,
        access_key: "accessKey".to_string(),
        settings: Settings::default(),
        throughput: Throughput {
            key: "throughputKey".to_string(),
            limit: 100,
        },
        ownership: Ownership::default(),
        oauth: None,
        record_metadata: RecordMetadata::default(),
    };

    db.collection("connections")
        .insert_one(
            bson::to_bson_with_options(&conn, Default::default())
                .expect("Unable to serialize connection"),
            None,
        )
        .await
        .unwrap();
    conn.id
}

async fn get_control_store(
    config: &EventCoreConfig,
    secrets_client: Arc<dyn SecretExt + Sync + Send>,
) -> MongoControlDataStore {
    MongoControlDataStore::new(config, secrets_client)
        .await
        .unwrap()
}

// TODO: Fix this test
// #[tokio::test]
// async fn test_send_to_destination() {
//     let docker = Docker::default();
//     let mongo = docker.run(Mongo);
//     let host_port = mongo.get_host_port_ipv4(27017);
//     let connection_string = format!("mongodb://127.0.0.1:{host_port}/?directConnection=true");

//     let config = EventCoreConfig::init_from_hashmap(&HashMap::from([
//         ("CONTROL_DATABASE_URL".to_string(), connection_string),
//         (
//             "CONTROL_DATABASE_NAME".to_string(),
//             Uuid::new_v4().to_string(),
//         ),
//     ]))
//     .unwrap();

//     let secret_key = "Stripe secret key";

//     let mut mock_server = Server::new_async().await;

//     let mock = mock_server
//         .mock("POST", "/api/customers")
//         .match_header("Authorization", format!("Bearer {secret_key}").as_str())
//         .with_status(200)
//         .with_body("Great success!")
//         .expect(1)
//         .create_async()
//         .await;

//     seed_db(&config, mock_server.url() + "/api").await;

//     #[derive(Clone)]
//     struct SecretsClient;
//     #[async_trait::async_trait]
//     impl CryptoExt for SecretsClient {
//         async fn decrypt(&self, _secret: &GetSecretRequest) -> Result<Value, IntegrationOSError> {
//             Ok(json!({
//                 "STRIPE_SECRET_KEY": "Stripe secret key"
//             }))
//         }
//         async fn encrypt(
//             &self,
//             _key: String,
//             _value: &serde_json::Value,
//         ) -> Result<Secret, IntegrationOSError> {
//             Ok(Secret::new(
//                 "encrypted_secret".into(),
//                 Some(SecretVersion::V1),
//                 "buildable_id".into(),
//                 None,
//             ))
//         }
//     }

//     let store = get_control_store(&config, Arc::new(SecretsClient)).await;

//     let mut pipeline: Pipeline = Faker.fake();
//     pipeline.destination.connection_key = "key".into();
//     pipeline.destination.platform = "stripe".into();
//     pipeline.destination.action = Action::Passthrough {
//         method: Method::POST,
//         path: "customers".into(),
//     };

//     let event = Faker.fake();

//     let name: String = Name().fake();
//     let email: String = FreeEmail().fake();

//     let result = store
//         .send_to_destination(
//             &event,
//             &pipeline,
//             Some(json!({
//                 "name": name,
//                 "email": email
//             })),
//         )
//         .await;

//     assert!(result.is_ok());
//     assert_eq!(result.unwrap(), "Great success!".to_string());

//     mock.assert_async().await;
// }
