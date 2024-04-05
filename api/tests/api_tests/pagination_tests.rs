use std::time::Duration;

use api::endpoints::{pipeline::CreatePipelineRequest, ReadResponse};
use fake::{Fake, Faker};
use http::{Method, StatusCode};
use integrationos_domain::common::Pipeline;
use serde_json::Value;
use tokio::time::sleep;

use crate::test_server::TestServer;

#[tokio::test]
async fn test_pagination() {
    let server = TestServer::new(false, None).await;

    let mut pipelines = vec![];
    for _ in 0..10 {
        let req: CreatePipelineRequest = Faker.fake();
        let res = server
            .send_request::<Value, Value>(
                "v1/pipelines",
                Method::POST,
                Some(&server.live_key),
                Some(&serde_json::to_value(&req).unwrap()),
            )
            .await
            .unwrap();
        assert_eq!(res.code, StatusCode::OK);

        let pipeline: Pipeline = serde_json::from_value(res.data).unwrap();
        let CreatePipelineRequest {
            name,
            key,
            source,
            destination,
            middleware,
            signature,
            ref config,
        } = req;

        assert_eq!(name, pipeline.name);
        assert_eq!(key, pipeline.key);
        assert_eq!(source, pipeline.source);
        assert_eq!(destination, pipeline.destination);
        assert_eq!(middleware, pipeline.middleware);
        assert_eq!(signature, pipeline.signature);
        assert_eq!(config, pipeline.config.as_ref().unwrap());

        pipelines.push(pipeline);
        sleep(Duration::from_millis(1)).await;
    }

    let pipelines: Vec<Pipeline> = pipelines.into_iter().rev().collect();

    check_response(&server, 1, 0, &pipelines[..1]).await;
    check_response(&server, 10, 0, &pipelines).await;
    check_response(&server, 0, 10, &pipelines[10..]).await;
    check_response(&server, 5, 0, &pipelines[..5]).await;
    check_response(&server, 5, 5, &pipelines[5..]).await;
    check_response(&server, 5, 10, &pipelines[10..]).await;
}

async fn check_response(server: &TestServer, limit: u64, skip: u64, pipelines: &[Pipeline]) {
    let res = server
        .send_request::<Value, Value>(
            &format!("v1/pipelines?limit={limit}&skip={skip}"),
            Method::GET,
            Some(&server.live_key),
            None,
        )
        .await
        .unwrap();
    assert_eq!(res.code, StatusCode::OK);

    let res: ReadResponse<Pipeline> = serde_json::from_value(res.data).unwrap();
    assert_eq!(&res.rows, pipelines);
    assert_eq!(res.limit, limit);
    assert_eq!(res.skip, skip);
    assert_eq!(res.total, 10);
}
