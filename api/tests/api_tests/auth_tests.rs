use super::test_server::{ApiResponse, TestServer, PUBLIC_PATHS};
use api::endpoints::event_access::CreateEventAccessPayloadWithOwnership;
use fake::{Fake, Faker};
use http::{
    header::{AUTHORIZATION, CONTENT_TYPE},
    Method, StatusCode,
};
use serde_json::{json, Value};

#[tokio::test]
async fn test_root() {
    let server = TestServer::new(None).await;

    let res = server
        .send_request::<Value, Value>("", Method::GET, None, None)
        .await
        .unwrap();
    assert_eq!(res.code, StatusCode::OK);
}

#[tokio::test]
async fn test_unauthorized() {
    let server = TestServer::new(None).await;

    for path in PUBLIC_PATHS {
        let res = server
            .send_request::<Value, Value>(&format!("v1/public/{path}"), Method::GET, None, None)
            .await
            .unwrap();
        assert_eq!(res.code, StatusCode::OK);
    }
}

#[tokio::test]
async fn test_404() {
    let server = TestServer::new(None).await;

    for method in [Method::GET, Method::POST, Method::DELETE] {
        let res = server
            .send_request::<Value, Value>("v1/invalid_path", method, Some(&server.live_key), None)
            .await
            .unwrap();

        assert_eq!(
            res,
            ApiResponse {
                code: StatusCode::NOT_FOUND,
                data: json!({"error": "Not found"})
            }
        );
    }
}
