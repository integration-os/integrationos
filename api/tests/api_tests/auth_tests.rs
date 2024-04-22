use api::endpoints::event_access::CreateEventAccessPayloadWithOwnership;
use fake::{Fake, Faker};
use http::{
    header::{AUTHORIZATION, CONTENT_TYPE},
    Method, StatusCode,
};
use serde_json::{json, Value};

use super::test_server::{ApiResponse, TestServer, PUBLIC_PATHS};

#[tokio::test]
async fn test_root() {
    let server = TestServer::new(false, None).await;

    let res = server
        .send_request::<Value, Value>("", Method::GET, None, None)
        .await
        .unwrap();
    assert_eq!(res.code, StatusCode::OK);
}

#[tokio::test]
async fn test_unauthorized() {
    let server = TestServer::new(false, None).await;

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
    let server = TestServer::new(false, None).await;

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

#[tokio::test]
async fn test_jwt() {
    let server = TestServer::new(false, None).await;

    let jwt_token = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJfaWQiOiI2NTc5ZDUxMGE2ZTQyMTAyMzM0NjI0ZjAiLCJlbWFpbCI6ImVtYWlsQHRlc3QuY29tIiwidXNlcm5hbWUiOiJ1c2VybmFtZSIsInVzZXJLZXkiOiJ1c2VyS2V5IiwiZmlyc3ROYW1lIjoiUGF1bCIsImxhc3ROYW1lIjoiSy4iLCJidWlsZGFibGVJZCI6ImJ1aWxkLTI2MTU4YWFlNzNjMDQ4YTU4YzdhNzU2NjcyNmU4OGY0IiwiY29udGFpbmVySWQiOiJjb250YWluZXItZDRmMGY4YjktMWE1Ni00ODQxLTg1OTctZmQzZDkwZGQ0OWI5IiwicG9pbnRlcnMiOlsiXzFfazFjbkI0Y1hGMzYtYUJJc2gtY1ZWTlZNZllGeE41MWlFTlQ1azlqcXFEbURWZlpJTjVVREhlN0JKRnJaUVJqTm54aEdaOUJNUGdlNjB6RVdVUnROaTUxdTIwdDJiYVJoQ3ZkYms5TkNIblNSV010WldhMmFlVW0wWUpreU1PNGNEUjdVUW5oVmNac3RqUEdfN0lfcV9ya015cjlwaFZoZ1VBa3BCaDVDTlQ2VDIwTDJGTFpoMFVtdldLVzloV3IzN0JWV19tb0hZODFZeUEiLCJfMV81WVU2Uk1kMHRwUVh3YnNvUWtHaUkzT1hPRlhrbms3TUVhaVdTQ1hoUWZKYzZ5N3RqZGROZGRjbXdWRjJmcTRSTktla0ZXRk80M0FMQWNJTVdIYkdYbW9IVVRaelV1eXhMalJ5MDI5Z0tGMlViRTFmTzZVRzR5QWhzbFBJMlpOZXNnT2NiakY1eUdwajdJbkdHNUFVck13NGY0bVdfR29FZVp1ZXBBd2E0WHhzNHB2TXd5d241djc1VzV0dFNJRGtLTHFqUlNUQlczaHpLUSJdLCJpc0J1aWxkYWJsZUNvcmUiOnRydWUsImlhdCI6MTcwMzEwODkwNCwiZXhwIjozMTU3NDYzMTA4OTA0LCJhdWQiOiJidWlsZGFibGUtdXNlcnMiLCJpc3MiOiJidWlsZGFibGUifQ.ecKXIGxXLWd6OearftRZVpGRhyDUVZFrYlwzhr-iG0A";

    let event_access: CreateEventAccessPayloadWithOwnership = Faker.fake();
    let event_access = serde_json::to_value(&event_access).unwrap();

    let res = server
        .send_request_with_headers::<Value, Value>(
            "v1/public/event-access/default",
            Method::POST,
            None,
            Some(&event_access),
            Some(
                vec![
                    (AUTHORIZATION.to_string(), format!("Bearer {jwt_token}")),
                    (CONTENT_TYPE.to_string(), "application/json".to_string()),
                ]
                .into_iter()
                .collect(),
            ),
        )
        .await
        .unwrap();

    assert_eq!(res.code, StatusCode::OK);
}
