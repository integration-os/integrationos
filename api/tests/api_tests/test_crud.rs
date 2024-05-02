use crate::test_server::TestServer;
use api::endpoints::{
    common_model, connection_definition, connection_model_definition, connection_model_schema,
    ReadResponse,
};
use fake::{Fake, Faker};
use http::{Method, StatusCode};
use integrationos_domain::{
    common_model::CommonModel, connection_definition::ConnectionDefinition,
    connection_model_definition::ConnectionModelDefinition,
    connection_model_schema::ConnectionModelSchema,
};
use serde_json::Value;
use std::collections::HashMap;

macro_rules! crud {
    ($(#[$m:meta])*, $test:ident, $model:ty, $path:ident, $endpoint:expr) => {
        $(#[$m])*
        async fn $test() {
            let server = TestServer::new(None).await;

            let payload: $path::CreateRequest = Faker.fake();
            let payload = serde_json::to_value(&payload).unwrap();

            const ENDPOINT: &str = $endpoint;

            let res = server
                .send_request::<Value, Value>(ENDPOINT, Method::POST, Some(&server.live_key), Some(&payload))
                .await
                .unwrap();

            assert_eq!(res.code, StatusCode::OK);

            let model: $model = serde_json::from_value(res.data).expect("Failed to deserialize model");

            let res = server
                .send_request::<Value, Value>(ENDPOINT, Method::GET, Some(&server.live_key), None)
                .await
                .unwrap();

            assert_eq!(res.code, StatusCode::OK);

            let get_models: ReadResponse<$model> = serde_json::from_value(res.data).unwrap();
            assert_eq!(get_models.rows.len(), 1);
            assert_eq!(get_models.rows[0], model);

            let payload: $path::CreateRequest = Faker.fake();
            let payload = serde_json::to_value(&payload).unwrap();

            let path = format!("{ENDPOINT}/{}", model.id);

            let res = server
                .send_request::<Value, Value>(&path, Method::PATCH, Some(&server.live_key), Some(&payload))
                .await;

            let res = res.unwrap();

            assert_eq!(res.code, StatusCode::OK);

            let res = server
                .send_request::<Value, Value>(&path, Method::DELETE, Some(&server.live_key), None)
                .await
                .unwrap();

            assert_eq!(res.code, StatusCode::OK);

            let deleted: $model = serde_json::from_value(res.data).unwrap();
            assert_eq!(deleted.id, model.id);

            let res = server
                .send_request::<Value, Value>(ENDPOINT, Method::GET, Some(&server.live_key), None)
                .await
                .unwrap();

            assert_eq!(res.code, StatusCode::OK);

            let get_models: ReadResponse<$model> = serde_json::from_value(res.data).unwrap();
            assert!(get_models.rows.is_empty());
        }
    };
}

crud!(
    #[ignore = "Until we have a JWT token generated we can not test this"],
    test_connection_definitions_crud,
    ConnectionDefinition,
    connection_definition,
    "v1/connection-definitions"
);

crud!(
    #[ignore = "Until we have a JWT token generated we can not test this"],
    test_connection_model_definitions_crud,
    ConnectionModelDefinition,
    connection_model_definition,
    "v1/connection-model-definitions"
);

crud!(
    #[ignore = "Until we have a JWT token generated we can not test this"],
    test_connection_model_schema_crud,
    ConnectionModelSchema,
    connection_model_schema,
    "v1/connection-model-schemas"
);

#[tokio::test]
#[ignore = "Until we have a JWT token generated we can not test this"]
async fn test_common_model_crud() {
    let server = TestServer::new(None).await;

    let payload: common_model::CreateRequest = Faker.fake();
    let payload = serde_json::to_value(&payload).unwrap();

    const ENDPOINT: &str = "v1/common-models";

    let res = server
        .send_request::<Value, Value>(
            ENDPOINT,
            Method::POST,
            Some(&server.live_key),
            Some(&payload),
        )
        .await
        .unwrap();

    assert_eq!(res.code, StatusCode::OK);

    let mut model: CommonModel = serde_json::from_value(res.data).unwrap();
    model.interface = HashMap::new();

    let res = server
        .send_request::<Value, Value>(ENDPOINT, Method::GET, Some(&server.live_key), None)
        .await
        .unwrap();

    assert_eq!(res.code, StatusCode::OK);

    let mut get_models: ReadResponse<CommonModel> = serde_json::from_value(res.data).unwrap();
    get_models
        .rows
        .iter_mut()
        .for_each(|x| x.interface = HashMap::new());

    assert_eq!(get_models.rows.len(), 1);
    assert_eq!(get_models.rows[0], model);

    let payload: common_model::CreateRequest = Faker.fake();
    let payload = serde_json::to_value(&payload).unwrap();

    let path = format!("{ENDPOINT}/{}", model.id);

    let res = server
        .send_request::<Value, Value>(&path, Method::PATCH, Some(&server.live_key), Some(&payload))
        .await;

    let res = res.unwrap();

    assert_eq!(res.code, StatusCode::OK);

    let res = server
        .send_request::<Value, Value>(&path, Method::DELETE, Some(&server.live_key), None)
        .await
        .unwrap();

    assert_eq!(res.code, StatusCode::OK);

    let mut deleted: CommonModel = serde_json::from_value(res.data).unwrap();
    deleted.interface = HashMap::new();

    assert_eq!(deleted.id, model.id);

    let res = server
        .send_request::<Value, Value>(ENDPOINT, Method::GET, Some(&server.live_key), None)
        .await
        .unwrap();

    assert_eq!(res.code, StatusCode::OK);

    let mut get_models: ReadResponse<CommonModel> = serde_json::from_value(res.data).unwrap();
    get_models
        .rows
        .iter_mut()
        .for_each(|x| x.interface = HashMap::new());

    assert!(get_models.rows.is_empty());
}
