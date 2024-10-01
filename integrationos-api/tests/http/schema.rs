use crate::context::TestServer;
use http::{Method, StatusCode};
use serde_json::Value;

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
