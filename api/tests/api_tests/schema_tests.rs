use api::endpoints::{connection_model_schema::CreateRequest, ReadResponse};
use fake::{Fake, Faker};
use http::{Method, StatusCode};
use integrationos_domain::{
    id::{prefix::IdPrefix, Id},
    {
        connection_model_schema::{ConnectionModelSchema, Mappings},
        environment::Environment,
        json_schema::JsonSchema,
    },
};
use serde_json::Value;

use crate::test_server::TestServer;

#[tokio::test]
async fn test_connection_model_schema_api() {
    let mut server = TestServer::new(None).await;

    let (_connection, conn_def) = server.create_connection(Environment::Live).await;

    let common_model_name = Faker.fake();

    let mut create_connection_model_schema: CreateRequest = Faker.fake();
    create_connection_model_schema.connection_definition_id = conn_def.id;
    create_connection_model_schema.mapping = Some(Mappings {
        from_common_model: String::new(),
        to_common_model: String::new(),
        common_model_name,
        common_model_id: Id::now(IdPrefix::ConnectionModelSchema),
        unmapped_fields: JsonSchema::default(),
    });

    let create_response = server
        .send_request::<CreateRequest, ConnectionModelSchema>(
            "v1/connection-model-schemas",
            Method::POST,
            None,
            Some(&create_connection_model_schema),
        )
        .await
        .unwrap();

    assert_eq!(create_response.code, StatusCode::OK);

    let public_connection_model_schema = server
        .send_request::<Value, ReadResponse<Value>>(
            format!(
                "v1/connection-model-schemas?connectionDefinitionId={}",
                conn_def.id
            )
            .as_str(),
            Method::GET,
            Some(&server.live_key),
            None,
        )
        .await
        .unwrap();

    assert_eq!(public_connection_model_schema.code, StatusCode::OK);

    let first_row = public_connection_model_schema
        .data
        .rows
        .first()
        .expect("No rows in response");

    let mapping = first_row.get("mapping").expect("No mapping in row");

    assert!(
        mapping.get("fromCommonModel").is_none(),
        "fromCommonModel should not be present"
    );
    assert!(
        mapping.get("toCommonModel").is_none(),
        "toCommonModel should not be present"
    );
    assert!(
        mapping.get("commonModelId").is_none(),
        "commonModelId should not be present"
    );
    assert!(
        mapping.get("unmappedFields").is_none(),
        "unmappedFields should not be present"
    );
}

#[tokio::test]
async fn test_connection_oauth_definition_schema_api() {
    let server = TestServer::new(None).await;
    let res = server
        .send_request::<Value, Value>(
            "v1/public/connection-oauth-definition-schema",
            Method::GET,
            None,
            None,
        )
        .await
        .unwrap();
    assert_eq!(res.code, StatusCode::OK);
}
