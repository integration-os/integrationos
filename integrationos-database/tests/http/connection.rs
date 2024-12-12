use crate::context::{TestServer, DOCKER, POSTGRES};
use http::{Method, StatusCode};
use integrationos_domain::{
    database::PostgresConfig, database_secret::DatabaseConnectionSecret, prefix::IdPrefix, Id,
    IntegrationOSError, Secret, SecretVersion, Unit,
};
use mockito::Server as MockServer;
use serde_json::Value;
use std::collections::HashMap;
use testcontainers_modules::postgres::Postgres;

#[tokio::test]
async fn test_execute_probe() -> Result<Unit, IntegrationOSError> {
    let mut mock_server = MockServer::new_async().await;
    let mock_uri = mock_server.url();

    let connection_id = Id::now(IdPrefix::Connection);

    let docker = DOCKER.get_or_init(Default::default);
    let postgres = POSTGRES.get_or_init(|| docker.run(Postgres::default()));
    let port = postgres.get_host_port_ipv4(5432);

    let database_secret = DatabaseConnectionSecret {
        namespace: "development-db-conns".to_string(),
        service_name: "service_name".to_string(),
        connection_id,
        postgres_config: PostgresConfig {
            postgres_username: "postgres".to_string(),
            postgres_password: "postgres".to_string(),
            postgres_port: port,
            postgres_name: "postgres".to_string(),
            postgres_host: "localhost".to_string(),
            postgres_ssl: false,
            postgres_timeout: 3000,
            postgres_pool_size: 4,
        },
    };

    let database_secret =
        serde_json::to_string(&database_secret).expect("Failed to serialize secret");

    let secret = Secret::new(
        database_secret,
        Some(SecretVersion::V2),
        "secret_id".to_string(),
        None,
    );

    let secret = serde_json::to_string(&secret).expect("Failed to serialize secret");

    let path = format!("/v1/admin/connection/{connection_id}");
    let secret_req = mock_server
        .mock("GET", path.as_str())
        .with_status(200)
        .with_body(secret)
        .create_async()
        .await;

    let server = TestServer::new(HashMap::from([
        ("CONNECTION_ID".to_string(), connection_id.to_string()),
        ("EMIT_URL".to_string(), mock_uri.clone()),
        ("CONNECTIONS_URL".to_string(), mock_uri),
    ]))
    .await?;

    // let server = TestServer::new(HashMap::new()).await?;
    let result = server
        .send_request::<Value, Value>("database/probe", Method::GET, None)
        .await?;

    assert_eq!(result.code, StatusCode::OK);

    let data = result.data.to_string();
    assert!(data.contains("[{\"?column?\":1}]"));
    secret_req.expect(1).assert_async().await;
    Ok(())
}

#[tokio::test]
async fn test_execute_raw() -> Result<Unit, IntegrationOSError> {
    let mut mock_server = MockServer::new_async().await;
    let mock_uri = mock_server.url();

    let connection_id = Id::now(IdPrefix::Connection);

    let docker = DOCKER.get_or_init(Default::default);
    let postgres = POSTGRES.get_or_init(|| docker.run(Postgres::default()));
    let port = postgres.get_host_port_ipv4(5432);

    let database_secret = DatabaseConnectionSecret {
        namespace: "development-db-conns".to_string(),
        service_name: "service_name".to_string(),
        connection_id,
        postgres_config: PostgresConfig {
            postgres_username: "postgres".to_string(),
            postgres_password: "postgres".to_string(),
            postgres_port: port,
            postgres_name: "postgres".to_string(),
            postgres_host: "localhost".to_string(),
            postgres_ssl: false,
            postgres_timeout: 3000,
            postgres_pool_size: 4,
        },
    };

    let database_secret =
        serde_json::to_string(&database_secret).expect("Failed to serialize secret");

    let secret = Secret::new(
        database_secret,
        Some(SecretVersion::V2),
        "secret_id".to_string(),
        None,
    );

    let secret = serde_json::to_string(&secret).expect("Failed to serialize secret");

    let path = format!("/v1/admin/connection/{connection_id}");
    let secret_req = mock_server
        .mock("GET", path.as_str())
        .with_status(200)
        .with_body(secret)
        .create_async()
        .await;

    let server = TestServer::new(HashMap::from([
        ("CONNECTION_ID".to_string(), connection_id.to_string()),
        ("EMIT_URL".to_string(), mock_uri.clone()),
        ("CONNECTIONS_URL".to_string(), mock_uri),
    ]))
    .await?;

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
    secret_req.expect(1).assert_async().await;

    Ok(())
}
