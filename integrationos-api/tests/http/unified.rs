use crate::context::TestServer;
use chrono::{Datelike, Utc};
use fake::{faker::filesystem::raw::DirPath, locales::EN, Fake, Faker};
use http::{
    header::{AUTHORIZATION, CONTENT_TYPE},
    Method, StatusCode,
};
use integrationos_api::logic::{
    connection_model_definition::CreateRequest as CreateConnectionModelDefinitionRequest,
    connection_model_schema::CreateRequest as CreateConnectionModelSchemaRequest,
    metrics::MetricResponse,
};
use integrationos_domain::{
    api_model_config::{AuthMethod, SamplesInput, SchemasInput},
    connection_model_definition::{ConnectionModelDefinition, CrudAction, CrudMapping},
    connection_model_schema::{ConnectionModelSchema, Mappings},
    environment::Environment,
    id::{prefix::IdPrefix, Id},
    SanitizedConnection,
};
use mockito::Mock;
use serde_json::Value;
use std::time::Duration;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_unified_api_get_many() {
    let mut server = TestServer::new(None).await;
    let (connection, _) = server.create_connection(Environment::Live).await;

    let name = "Model".to_string();

    let mock = create_connection_model_definition(
        &mut server,
        &connection,
        CrudMapping {
            action: CrudAction::GetMany,
            common_model_name: name.clone(),
            from_common_model: Some(
                "function mapCrudRequest(data) {
                data.queryParams = undefined;
                return data;
            }"
                .to_string(),
            ),
            to_common_model: Some(
                "function mapCrudRequest(data) {
                data.queryParams = undefined;
                return data;
            }"
                .to_string(),
            ),
        },
    )
    .await;

    let res = server
        .send_request_with_headers::<Value, Value>(
            &format!("v1/unified/{}?foo=bar", name.to_lowercase()),
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
        .unwrap();

    assert_eq!(res.code, StatusCode::OK);

    mock.assert_async().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_unified_api_get_one() {
    let mut server = TestServer::new(None).await;
    let (connection, _) = server.create_connection(Environment::Live).await;

    let name = "Model".to_string();

    let id: String = Faker.fake();

    let mock = create_connection_model_definition(
        &mut server,
        &connection,
        CrudMapping {
            action: CrudAction::GetOne,
            common_model_name: name.clone(),
            from_common_model: Some(
                "function mapCrudRequest(data) {
                return data;
            }"
                .to_string(),
            ),
            to_common_model: Some(
                "function mapCrudRequest(data) {
                data.queryParams = undefined;
                return data;
            }"
                .to_string(),
            ),
        },
    )
    .await;

    let res = server
        .send_request_with_headers::<Value, Value>(
            &format!("v1/unified/{}/{id}", name.to_lowercase()),
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
        .unwrap();

    assert_eq!(res.code, StatusCode::OK);

    mock.assert_async().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_unified_api_get_count() {
    let mut server = TestServer::new(None).await;
    let (connection, _) = server.create_connection(Environment::Live).await;

    let name = "Model".to_string();

    let mock = create_connection_model_definition(
        &mut server,
        &connection,
        CrudMapping {
            action: CrudAction::GetCount,
            common_model_name: name.clone(),
            from_common_model: Some(
                "function mapCrudRequest(data) {
                return data;
            }"
                .to_string(),
            ),
            to_common_model: Some(
                "function mapCrudRequest(data) {
                data.queryParams = undefined;
                return data;
            }"
                .to_string(),
            ),
        },
    )
    .await;

    let res = server
        .send_request_with_headers::<Value, Value>(
            &format!("v1/unified/{}/count", name.to_lowercase()),
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
        .unwrap();

    assert_eq!(res.code, StatusCode::OK);

    mock.assert_async().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_unified_api_update() {
    let mut server = TestServer::new(None).await;
    let (connection, _) = server.create_connection(Environment::Live).await;

    let name = "Model".to_string();

    let id: String = Faker.fake();

    let mock = create_connection_model_definition(
        &mut server,
        &connection,
        CrudMapping {
            action: CrudAction::Update,
            common_model_name: name.clone(),
            from_common_model: Some(
                "function mapCrudRequest(data) {
                return data;
            }"
                .to_string(),
            ),
            to_common_model: Some(
                "function mapCrudRequest(data) {
                data.queryParams = undefined;
                return data;
            }"
                .to_string(),
            ),
        },
    )
    .await;

    let payload: Value = Faker.fake();

    let res = server
        .send_request_with_headers::<Value, Value>(
            &format!("v1/unified/{}/{id}", name.to_lowercase()),
            Method::PATCH,
            Some(&server.live_key),
            Some(&payload),
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
        .expect("Failed to send request");

    assert_eq!(res.code, StatusCode::OK);

    mock.assert_async().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_unified_api_delete() {
    let mut server = TestServer::new(None).await;
    let (connection, _) = server.create_connection(Environment::Live).await;

    let name = "Model".to_string();

    let id: String = Faker.fake();

    let mock = create_connection_model_definition(
        &mut server,
        &connection,
        CrudMapping {
            action: CrudAction::Delete,
            common_model_name: name.clone(),
            from_common_model: Some(
                "function mapCrudRequest(data) {
                return data;
            }"
                .to_string(),
            ),
            to_common_model: Some(
                "function mapCrudRequest(data) {
                data.queryParams = undefined;
                return data;
            }"
                .to_string(),
            ),
        },
    )
    .await;

    let res = server
        .send_request_with_headers::<Value, Value>(
            &format!("v1/unified/{}/{id}", name.to_lowercase()),
            Method::DELETE,
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
        .unwrap();

    assert_eq!(res.code, StatusCode::OK);

    mock.assert_async().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_unified_api_create() {
    let mut server = TestServer::new(None).await;
    let (connection, _) = server.create_connection(Environment::Live).await;

    let name = "Model".to_string();

    let mock = create_connection_model_definition(
        &mut server,
        &connection,
        CrudMapping {
            action: CrudAction::Create,
            common_model_name: name.clone(),
            from_common_model: Some(
                "function mapCrudRequest(data) {
                return data;
            }"
                .to_string(),
            ),
            to_common_model: Some(
                "function mapCrudRequest(data) {
                data.queryParams = undefined;
                return data;
            }"
                .to_string(),
            ),
        },
    )
    .await;

    let payload: Value = Faker.fake();

    let res = server
        .send_request_with_headers::<Value, Value>(
            &format!("v1/unified/{}", name.to_lowercase()),
            Method::POST,
            Some(&server.live_key),
            Some(&payload),
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
        .unwrap();

    assert_eq!(res.code, StatusCode::OK);

    mock.assert_async().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_unified_metrics() {
    let mut server = TestServer::new(None).await;
    let (connection, _) = server.create_connection(Environment::Live).await;

    let name = "Model".to_string();

    let mock = create_connection_model_definition(
        &mut server,
        &connection,
        CrudMapping {
            action: CrudAction::Create,
            common_model_name: name.clone(),
            from_common_model: None,
            to_common_model: None,
        },
    )
    .await;

    let payload: Value = Faker.fake();

    let res = server
        .send_request_with_headers::<Value, Value>(
            &format!("v1/unified/{}", name.to_lowercase()),
            Method::POST,
            Some(&server.live_key),
            Some(&payload),
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
        .expect("Failed to send request");

    assert_eq!(res.code, StatusCode::OK);
    mock.assert_async().await;

    tokio::time::sleep(Duration::from_millis(100)).await;

    let res = server
        .send_request::<(), MetricResponse>("v1/metrics", Method::GET, Some(&server.live_key), None)
        .await
        .unwrap();

    assert_eq!(res.data.count, 1);

    let res = server
        .send_request_with_headers::<Value, Value>(
            &format!("v1/unified/{}", name.to_lowercase()),
            Method::POST,
            Some(&server.live_key),
            Some(&payload),
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
        .unwrap();
    assert_eq!(res.code, StatusCode::OK);

    tokio::time::sleep(Duration::from_millis(100)).await;

    let res = server
        .send_request::<(), MetricResponse>(
            format!("v1/metrics/{}", connection.ownership.client_id).as_str(),
            Method::GET,
            Some(&server.live_key),
            None,
        )
        .await
        .unwrap();

    assert_eq!(res.data.count, 2);

    let date = Utc::now();
    let day = date.day();
    let month = date.month();
    let year = date.year();
    let daily_key = format!("{year}-{month:02}-{day:02}");
    let monthly_key = format!("{year}-{month:02}");

    let res = server
        .send_request::<(), MetricResponse>(
            format!(
                "v1/metrics/{}?day={daily_key}",
                connection.ownership.client_id
            )
            .as_str(),
            Method::GET,
            Some(&server.live_key),
            None,
        )
        .await
        .unwrap();

    assert_eq!(res.data.count, 2);

    let res = server
        .send_request::<(), MetricResponse>(
            format!(
                "v1/metrics/{}?month={monthly_key}&apiType=unified",
                connection.ownership.client_id
            )
            .as_str(),
            Method::GET,
            Some(&server.live_key),
            None,
        )
        .await
        .unwrap();

    assert_eq!(res.data.count, 2);

    let res = server
        .send_request::<(), MetricResponse>(
            format!("v1/metrics?platform={}", connection.platform).as_str(),
            Method::GET,
            Some(&server.live_key),
            None,
        )
        .await
        .unwrap();

    assert_eq!(res.data.count, 2);

    let res = server
        .send_request::<(), MetricResponse>(
            "v1/metrics?apiType=passthrough",
            Method::GET,
            Some(&server.live_key),
            None,
        )
        .await
        .unwrap();

    assert_eq!(res.data.count, 0);
}

async fn create_connection_model_definition(
    server: &mut TestServer,
    connection: &SanitizedConnection,
    mapping: CrudMapping,
) -> Mock {
    let secret_key = Faker.fake::<String>();
    let url_path: String = DirPath(EN).fake();
    let path: String = Faker.fake();
    let response_body = format!("{{\"id\": \"{}\"}}", Faker.fake::<String>());

    let mock = server
        .mock_server
        .mock("GET", format!("{url_path}/{path}").as_str())
        .match_header(
            AUTHORIZATION.as_str(),
            format!("Bearer {secret_key}").as_str(),
        )
        .expect(1)
        .with_status(200)
        .with_body(response_body.clone())
        .create_async()
        .await;

    let create_model_definition_payload = CreateConnectionModelDefinitionRequest {
        id: None,
        connection_platform: connection.platform.to_string(),
        connection_definition_id: connection.connection_definition_id,
        platform_version: connection.record_metadata.version.to_string(),
        title: Faker.fake(),
        name: Faker.fake(),
        model_name: Faker.fake(),
        action_name: mapping.action.clone(),
        base_url: server.mock_server.url() + &url_path,
        path,
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
        test_connection_payload: None,
        mapping: Some(mapping.clone()),
        supported: Some(true),
        active: Some(true),
    };

    let create_model_definition_response = server
        .send_request::<CreateConnectionModelDefinitionRequest, ConnectionModelDefinition>(
            "v1/connection-model-definitions",
            Method::POST,
            Some(&server.live_key),
            Some(&create_model_definition_payload),
        )
        .await
        .unwrap();

    assert_eq!(create_model_definition_response.code, StatusCode::OK);

    let mut schema: CreateConnectionModelSchemaRequest = Faker.fake();
    schema.connection_platform = connection.platform.to_string();
    schema.mapping = Some(Mappings {
        from_common_model: "function mapFromCommonModel(data) { return data; }".to_string(),
        to_common_model: "function mapToCommonModel(data) { return data; }".to_string(),
        common_model_name: mapping.common_model_name.clone(),
        common_model_id: Id::now(IdPrefix::CommonModel),
        unmapped_fields: Default::default(),
    });

    let res = server
        .send_request::<CreateConnectionModelSchemaRequest, ConnectionModelSchema>(
            "v1/connection-model-schemas",
            Method::POST,
            Some(&server.live_key),
            Some(&schema),
        )
        .await
        .unwrap();

    assert_eq!(res.code, StatusCode::OK);

    mock
}
