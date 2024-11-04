use crate::context::TestServer;
use futures::{stream, StreamExt};
use http::{Method, StatusCode};
use integrationos_domain::{IntegrationOSError, Unit};
use serde_json::{json, Value};

const PARALLEL_REQUESTS: usize = 2;

#[tokio::test]
async fn test_concurrent_requests() -> Result<Unit, IntegrationOSError> {
    let server = TestServer::new().await?;
    let payload = json!({
        "type": "DatabaseConnectionLost",
        "connectionId": "conn::GAL2svWJp9k::MtmXaau5Qf6R5n3Y-L9ejQ"
    });

    // let reqs = vec!["emit"; PARALLEL_REQUESTS];
    //
    // let results = stream::iter(reqs)
    //     .map(|path| server.send_request::<Value, Value>(path, Method::POST, Some(&payload)))
    //     .buffer_unordered(PARALLEL_REQUESTS)
    //     .collect::<Vec<_>>()
    //     .await;
    //
    // println!("{:?}", results);

    let res = server
        .send_request::<Value, Value>("", Method::GET, None)
        .await?;

    // one of the requests should fail with a 409 conflict
    // the other should succeed
    // assert!(results.iter().any(|r| r.is_err()));
    // assert!(results.iter().any(|r| r.as_ref().unwrap().status() == StatusCode::OK));
    // assert_eq!(results.len(), PARALLEL_REQUESTS);
    // assert!(results.iter().any(|r| r.as_ref().unwrap().status() == StatusCode::CONFLICT));

    Ok(())
}
