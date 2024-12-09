use crate::context::TestServer;
use http::{Method, StatusCode};
use integrationos_domain::{IntegrationOSError, Unit};
use serde_json::Value;
use std::collections::HashMap;

#[tokio::test]
async fn test_execute_probe() -> Result<Unit, IntegrationOSError> {
    let server = TestServer::new(HashMap::new()).await?;
    let result = server
        .send_request::<Value, Value>("database/probe", Method::GET, None)
        .await?;

    assert_eq!(result.code, StatusCode::OK);

    let data = result.data.to_string();
    assert!(data.contains("[{\"?column?\":1}]"));
    Ok(())
}

#[tokio::test]
async fn test_execute_raw() -> Result<Unit, IntegrationOSError> {
    let server = TestServer::new(HashMap::new()).await?;

    let create_query =
        "CREATE TABLE IF NOT EXISTS users (id BIGSERIAL PRIMARY KEY, name TEXT NOT NULL);";
    let insert_query = "INSERT INTO users (id, name) VALUES (1, 'John');";
    let select_query = "SELECT * FROM users;";
    let drop_query = "DROP TABLE users;";

    let path = format!("database?query={}", create_query);
    let create_result = server
        .send_request::<Value, Value>(&path, Method::POST, None)
        .await?;
    assert_eq!(create_result.code, StatusCode::OK);

    let path = format!("database?query={}", insert_query);
    let insert_result = server
        .send_request::<Value, Value>(&path, Method::POST, None)
        .await?;
    assert_eq!(insert_result.code, StatusCode::OK);

    let path = format!("database?query={}", select_query);
    let select_result = server
        .send_request::<Value, Value>(&path, Method::POST, None)
        .await?;
    assert_eq!(select_result.code, StatusCode::OK);
    let data = select_result
        .data
        .as_array()
        .expect("Failed to get array")
        .first()
        .expect("Failed to get first element");

    let name = data.as_object().expect("Failed to get object")["name"]
        .as_str()
        .expect("Failed to get name");
    assert_eq!(name, "John");

    let id = data.as_object().expect("Failed to get object")["id"]
        .as_i64()
        .expect("Failed to get id");
    assert_eq!(id, 1);

    let path = format!("database?query={}", drop_query);
    let drop_result = server
        .send_request::<Value, Value>(&path, Method::POST, None)
        .await?;
    assert_eq!(drop_result.code, StatusCode::OK);

    // Test that the table is dropped
    let path = format!("database?query={}", select_query);
    let select_result = server
        .send_request::<Value, Value>(&path, Method::POST, None)
        .await?;
    assert_eq!(select_result.code, StatusCode::BAD_REQUEST);

    Ok(())
}
