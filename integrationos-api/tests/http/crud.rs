use crate::context::TestServer;
use crate::worker::gateway::TestGateway;
use fake::{Fake, Faker};
use http::{Method, StatusCode};
use integrationos_api::logic::{common_model, ReadResponse};
use integrationos_api::logic::{
    connection_definition, connection_model_definition, connection_model_schema,
};
use integrationos_domain::{
    common_model::CommonModel, connection_definition::ConnectionDefinition,
    connection_model_definition::ConnectionModelDefinition,
    connection_model_schema::ConnectionModelSchema,
};
use integrationos_domain::{
    common_model::{DataType, Expandable, Field},
    json_schema::JsonSchema,
};
use serde_json::{json, Value};
use std::{collections::HashMap, ops::Deref};

#[tokio::test]
async fn test_get_events() {
    let server = TestServer::new(None).await;

    let gateway = TestGateway::new(&server.config).await;

    let payload = json!({"foo":"bar"});

    let event_response = gateway
        .emit_event(&server.live_key, "name", &payload)
        .await
        .unwrap();
    assert_eq!(event_response.code, StatusCode::OK);

    let res = server
        .send_request::<Value, Value>("v1/events", Method::GET, Some(&server.live_key), None)
        .await
        .unwrap();

    assert_eq!(res.code, StatusCode::OK);
    let res: ReadResponse<Value> = serde_json::from_value(res.data).unwrap();
    let array = res.rows;
    assert_eq!(array.len(), 1);
    assert_eq!(array[0]["body"], payload.to_string());
}

#[tokio::test]
async fn test_get_expanded_common_model() {
    let server = TestServer::new(None).await;

    let reference: String = Faker.fake();

    let base = common_model::CreateRequest {
        id: None,
        name: Faker.fake(),
        version: Faker.fake(),
        fields: vec![
            Field {
                name: Faker.fake(),
                datatype: DataType::Expandable(Expandable::Unexpanded {
                    reference: reference.clone(),
                }),
                required: true,
                description: Faker.fake(),
            },
            Field {
                name: Faker.fake(),
                datatype: DataType::Array {
                    element_type: Box::new(DataType::Expandable(Expandable::Unexpanded {
                        reference: reference.clone(),
                    })),
                },
                required: true,
                description: Faker.fake(),
            },
        ],
        category: Faker.fake(),
        sample: json!({}),
        primary: false,
    };

    let expandable = common_model::CreateRequest {
        id: None,
        name: reference.clone(),
        version: Faker.fake(),
        fields: vec![],
        category: Faker.fake(),
        sample: json!({}),
        primary: false,
    };

    let res = server
        .send_request::<Value, Value>(
            "v1/common-models",
            Method::POST,
            Some(&server.live_key),
            Some(&serde_json::to_value(expandable).unwrap()),
        )
        .await
        .unwrap();
    assert_eq!(res.code, StatusCode::OK);

    let expandable: CommonModel = serde_json::from_value(res.data).unwrap();

    let res = server
        .send_request::<Value, Value>(
            "v1/common-models",
            Method::POST,
            Some(&server.live_key),
            Some(&serde_json::to_value(base).unwrap()),
        )
        .await
        .unwrap();
    assert_eq!(res.code, StatusCode::OK);

    let base: CommonModel = serde_json::from_value(res.data).unwrap();
    assert!(matches!(
        base.fields[0].datatype,
        DataType::Expandable(Expandable::Unexpanded { .. })
    ));
    let DataType::Array { ref element_type } = base.fields[1].datatype else {
        panic!("Incorrect datatype");
    };
    assert!(matches!(
        element_type.deref(),
        DataType::Expandable(Expandable::Unexpanded { .. })
    ));

    let res = server
        .send_request::<Value, Value>(
            &format!("v1/common-models/{}/expand", base.id),
            Method::GET,
            Some(&server.live_key),
            None,
        )
        .await
        .unwrap();
    assert_eq!(res.code, StatusCode::OK);

    let expanded: CommonModel = serde_json::from_value(res.data).unwrap();
    assert_eq!(expanded.name, base.name);
    assert_eq!(expanded.category, base.category);
    assert_eq!(expanded.fields[0].name, base.fields[0].name);
    assert_eq!(expanded.fields[1].name, base.fields[1].name);

    let DataType::Expandable(Expandable::Expanded {
        reference: ref new_ref,
        ref model,
    }) = expanded.fields[0].datatype
    else {
        panic!("Incorrect datatype");
    };

    assert_eq!(new_ref, &reference);
    let mut new_model = model.clone();
    new_model.interface = HashMap::new();
    assert_eq!(new_model, expandable);

    let DataType::Array { ref element_type } = expanded.fields[1].datatype else {
        panic!("Incorrect datatype");
    };
    let DataType::Expandable(Expandable::Expanded {
        reference: new_ref,
        ref model,
    }) = element_type.deref()
    else {
        panic!("Incorrect datatype");
    };
    assert_eq!(new_ref, &reference);
    // it is expected that interface is empty on the right side as it is created on the server
    // as a after effect of the create request
    let mut new_model = model.clone();
    new_model.interface = HashMap::new();
    assert_eq!(new_model, expandable);

    let res = server
        .send_request::<Value, JsonSchema>(
            &format!("v1/common-models/{}/schema", base.id),
            Method::GET,
            Some(&server.live_key),
            None,
        )
        .await
        .unwrap();
    assert_eq!(res.code, StatusCode::OK);
}

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
    #[tokio::test],
    test_connection_definitions_crud,
    ConnectionDefinition,
    connection_definition,
    "v1/connection-definitions"
);

crud!(
    #[tokio::test],
    test_connection_model_definitions_crud,
    ConnectionModelDefinition,
    connection_model_definition,
    "v1/connection-model-definitions"
);

crud!(
    #[tokio::test],
    test_connection_model_schema_crud,
    ConnectionModelSchema,
    connection_model_schema,
    "v1/connection-model-schemas"
);

#[tokio::test]
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
