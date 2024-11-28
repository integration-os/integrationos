use crate::context::TestServer;
use futures::{stream, StreamExt};
use http::{
    header::{ACCEPT, AUTHORIZATION, HOST},
    Method, StatusCode,
};
use integrationos_domain::{prefix::IdPrefix, Id, IntegrationOSError, Unit};
use integrationos_emit::logic::emitter::EntityIdResponse;
use mockito::Matcher;
use serde_json::{json, Value};
use std::{collections::HashMap, time::Duration};

const PARALLEL_REQUESTS: usize = 10;

#[tokio::test]
async fn test_concurrent_requests() -> Result<Unit, IntegrationOSError> {
    let server = TestServer::new(true).await?;
    let payload = json!({
        "type": "DatabaseConnectionLost",
        "connectionId": "conn::GAL2svWJp9k::MtmXaau5Qf6R5n3Y-L9ejQ"
    });

    let response = server
        .send_request::<Value, EntityIdResponse>("v1/emit", Method::POST, Some(&payload), None)
        .await;

    assert!(response.is_ok());

    let headers = HashMap::from_iter(vec![(
        "x-integrationos-idempotency-key".to_string(),
        response
            .expect("Failed to get response")
            .data
            .entity_id
            .to_string(),
    )]);

    let reqs = vec!["v1/emit"; PARALLEL_REQUESTS];

    let results = stream::iter(reqs)
        .map(|path| {
            server.send_request::<Value, Value>(path, Method::POST, Some(&payload), Some(&headers))
        })
        .buffer_unordered(PARALLEL_REQUESTS)
        .collect::<Vec<_>>()
        .await;

    assert_eq!(results.len(), PARALLEL_REQUESTS);
    let status_codes = results
        .into_iter()
        .map(|r| r.expect("Failed to send request").code)
        .collect::<Vec<_>>();

    assert!(status_codes.iter().any(|c| c == &StatusCode::CONFLICT));
    assert!(status_codes.iter().any(|c| c == &StatusCode::OK));

    assert_eq!(
        status_codes
            .iter()
            .filter(|c| c == &&StatusCode::CONFLICT)
            .count(),
        PARALLEL_REQUESTS - 1
    );
    assert_eq!(
        status_codes
            .iter()
            .filter(|c| c == &&StatusCode::OK)
            .count(),
        1
    );

    tokio::time::sleep(Duration::from_secs(10)).await;

    Ok(())
}

#[tokio::test]
async fn test_event_processed() -> Result<Unit, IntegrationOSError> {
    let mut server = TestServer::new(true).await?;

    let id = Id::now(IdPrefix::Connection).to_string();
    let payload = json!({
        "type": "DatabaseConnectionLost",
        "connectionId": id.clone()
    });
    let path = format!("/v1/event-callbacks/database-connection-lost/{}", id);
    let mock_server = server
        .mock_server
        .mock("POST", path.as_str())
        .match_header(AUTHORIZATION, Matcher::Any)
        .match_header(ACCEPT, "*/*")
        .match_header(HOST, server.mock_server.host_with_port().as_str())
        .with_status(200)
        .with_body("{}")
        .with_header("content-type", "application/json")
        .create_async()
        .await;

    let res = server
        .send_request::<Value, Value>("v1/emit", Method::POST, Some(&payload), None)
        .await
        .expect("Failed to send request");

    assert_eq!(res.code, StatusCode::OK);

    // Giving it some time for the commit to happen
    tokio::time::sleep(Duration::from_secs(10)).await;

    mock_server.expect_at_most(1).assert_async().await;

    let id = Id::now(IdPrefix::Connection).to_string();
    let payload = json!({
        "type": "DatabaseConnectionLost",
        "connectionId": id.clone()
    });
    let path = format!("/v1/event-callbacks/database-connection-lost/{}", id);
    let mock_server = server
        .mock_server
        .mock("POST", path.as_str())
        .match_header(AUTHORIZATION, Matcher::Any)
        .match_header(ACCEPT, "*/*")
        .match_header(HOST, server.mock_server.host_with_port().as_str())
        .with_status(500)
        .with_body("{}")
        .with_header("content-type", "application/json")
        .create_async()
        .await;

    let res = server
        .send_request::<Value, Value>("v1/emit", Method::POST, Some(&payload), None)
        .await
        .expect("Failed to send request");

    assert_eq!(res.code, StatusCode::OK);

    tokio::time::sleep(Duration::from_secs(3)).await;

    mock_server.expect_at_least(3).assert_async().await;
    Ok(())
}
