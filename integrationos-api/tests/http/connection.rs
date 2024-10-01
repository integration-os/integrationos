use crate::context::TestServer;
use http::{Method, StatusCode};
use serde_json::Value;

#[tokio::test]
async fn test_connection_data_models_api() {
    let server = TestServer::new(None).await;
    let res = server
        .send_request::<Value, Value>("v1/public/connection-data", Method::GET, None, None)
        .await
        .unwrap();
    assert_eq!(res.code, StatusCode::OK);
}
