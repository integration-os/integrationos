use crate::{
    context::TestServer,
    worker::{event::TestCore, gateway::TestGateway},
};
use fake::{Fake, Faker};
use http::{Method, StatusCode};
use integrationos_api::logic::{pipeline::CreatePipelineRequest, ReadResponse};
use integrationos_domain::{
    connection_model_definition::PlatformInfo, destination::Action, environment::Environment,
    Transaction,
};
use serde_json::{json, Value};
use std::time::Duration;

#[tokio::test(flavor = "multi_thread")]
async fn test_event_core() {
    let mut server = TestServer::new(None).await;

    let (connection, conn_def) = server.create_connection(Environment::Live).await;

    let event_name: String = Faker.fake();

    let mut pipeline: CreatePipelineRequest = Faker.fake();
    pipeline.source.group = connection.group;
    pipeline.source.r#type = server.live_access_key.data.event_type.clone();
    pipeline.source.events = vec![event_name.clone()];
    pipeline.middleware = vec![];
    pipeline.destination = Faker.fake();
    let PlatformInfo::Api(api_config) = conn_def.platform_info;
    pipeline.destination.platform = connection.platform.clone();
    pipeline.destination.connection_key = connection.key;
    pipeline.destination.action = Action::Passthrough {
        method: conn_def.action,
        path: api_config.path.into(),
    };

    let payload = serde_json::to_value(&pipeline).unwrap();

    server
        .send_request::<Value, Value>(
            "v1/pipelines",
            Method::POST,
            Some(&server.live_key),
            Some(&payload),
        )
        .await
        .unwrap();

    let gateway = TestGateway::new(&server.config).await;
    let core = TestCore::new(
        &server.config,
        &gateway.config,
        server.secrets_client.clone(),
    )
    .await;

    let payload = json!({"foo":"bar"});

    let event_response = gateway
        .emit_event(&server.live_key, &event_name, &payload)
        .await
        .unwrap();
    assert_eq!(event_response.code, StatusCode::OK);

    core.event_completed().await;

    tokio::time::sleep(Duration::from_millis(100)).await;

    let res = server
        .send_request::<Value, Value>("v1/transactions", Method::GET, Some(&server.live_key), None)
        .await
        .unwrap();

    assert!(res.code.is_success());

    let txs: ReadResponse<Transaction> = serde_json::from_value(res.data).unwrap();
    assert_eq!(txs.rows.len(), 2);
}
