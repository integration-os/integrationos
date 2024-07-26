use crate::test_server::TestServer;
use fake::{faker::filesystem::raw::DirPath, locales::EN, Fake, Faker};
use http::{
    header::{AUTHORIZATION, CONTENT_TYPE},
    Method, StatusCode,
};
use integrationos_api::logic::connection_model_definition::CreateRequest as CreateConnectionModelDefinitionRequest;
use integrationos_domain::{
    api_model_config::{AuthMethod, SamplesInput, SchemasInput},
    connection_model_definition::CrudAction,
    environment::Environment,
};
use mockito::Server;
use serde_json::Value;

#[tokio::test]
async fn test_passthrough_api() {
    let mut server = TestServer::new(None).await;
    let (connection, conn_def) = server.create_connection(Environment::Live).await;

    let mut mock_server = Server::new_async().await;
    let secret_key = Faker.fake::<String>();
    let url_path: String = DirPath(EN).fake();
    let response_body = format!("{{\"id\": \"{}\"}}", Faker.fake::<String>());

    let mock = mock_server
        .mock("GET", format!("{url_path}/customers").as_str())
        .match_header(
            AUTHORIZATION.as_str(),
            format!("Bearer {secret_key}").as_str(),
        )
        .expect(1)
        .with_status(200)
        .with_body(response_body.clone())
        .create();

    let create_model_definition_payload = CreateConnectionModelDefinitionRequest {
        _id: None,
        connection_platform: connection.platform.to_string(),
        connection_definition_id: conn_def.id,
        platform_version: conn_def.record_metadata.version.to_string(),
        title: Faker.fake(),
        name: Faker.fake(),
        model_name: Faker.fake(),
        action_name: CrudAction::Create,
        base_url: mock_server.url() + &url_path,
        path: "customers".to_string(),
        auth_method: AuthMethod::BearerToken {
            value: secret_key.to_string(),
        },
        http_method: http::Method::GET,
        headers: None,
        query_params: None,
        extractor_config: None,
        version: "1.0.0".parse().unwrap(),
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
        paths: None,
        responses: vec![],
        is_default_crud_mapping: None,
        mapping: None,
        supported: Some(true),
    };

    let create_model_definition_response = server
        .send_request::<Value, Value>(
            "v1/connection-model-definitions",
            Method::POST,
            Some(&server.live_key),
            Some(&serde_json::to_value(&create_model_definition_payload).unwrap()),
        )
        .await
        .unwrap();

    assert_eq!(create_model_definition_response.code, StatusCode::OK);

    let unverified_create_model_definition_payload = CreateConnectionModelDefinitionRequest {
        supported: Some(false),
        path: "invoices".to_string(),
        ..create_model_definition_payload.clone()
    };

    let create_unverified_model_definition_response = server
        .send_request::<Value, Value>(
            "v1/connection-model-definitions",
            Method::POST,
            Some(&server.live_key),
            Some(&serde_json::to_value(&unverified_create_model_definition_payload).unwrap()),
        )
        .await
        .unwrap();

    assert_eq!(
        create_unverified_model_definition_response.code,
        StatusCode::OK
    );

    // Test a passthrough API call for a verified connection model definition
    let call_universal_api = server
        .send_request_with_headers::<Value, Value>(
            "v1/passthrough/customers",
            Method::GET,
            Some(&server.live_key),
            None,
            Some(
                vec![
                    (CONTENT_TYPE.to_string(), "application/json".to_string()),
                    (
                        "x-integrationos-connection-key".to_string(),
                        connection.key.to_string(),
                    ),
                ]
                .into_iter()
                .collect(),
            ),
        )
        .await
        .expect("Failed to call passthrough API");

    assert_eq!(
        call_universal_api.data,
        serde_json::from_str::<Value>(&response_body).unwrap()
    );

    // Test a passthrough API call for an unverified connection model definition
    let call_unverified_passthrough_endpoint = server
        .send_request_with_headers::<Value, Value>(
            "v1/passthrough/invoices",
            Method::GET,
            Some(&server.live_key),
            None,
            Some(
                vec![
                    (CONTENT_TYPE.to_string(), "application/json".to_string()),
                    (
                        "x-integrationos-connection-key".to_string(),
                        connection.key.to_string(),
                    ),
                ]
                .into_iter()
                .collect(),
            ),
        )
        .await
        .expect("Failed to call the passthrough API");

    assert_eq!(
        call_unverified_passthrough_endpoint.code,
        StatusCode::NOT_FOUND
    );

    mock.assert_async().await;
}
