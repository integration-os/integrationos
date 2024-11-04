use std::collections::HashMap;

use crate::context::TestServer;
use futures::{stream, StreamExt};
use http::{Method, StatusCode};
use integrationos_domain::{IntegrationOSError, Unit};
use serde_json::{json, Value};
use uuid::Uuid;

const PARALLEL_REQUESTS: usize = 10;

#[tokio::test]
async fn test_concurrent_requests() -> Result<Unit, IntegrationOSError> {
    let server = TestServer::new().await?;
    let payload = json!({
        "type": "DatabaseConnectionLost",
        "connectionId": "conn::GAL2svWJp9k::MtmXaau5Qf6R5n3Y-L9ejQ"
    });

    let headers = HashMap::from_iter(vec![(
        "x-integrationos-idempotency-key".to_string(),
        Uuid::new_v4().to_string(),
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

    Ok(())
}
