// use super::*;
//     use crate::domain::config::StorageConfig;
//     use envconfig::Envconfig;
//     use integrationos_domain::IntegrationOSError;
//     use serde_json::Number;
//     use std::sync::OnceLock;
//     use testcontainers_modules::{
//         postgres::Postgres,
//         testcontainers::{clients::Cli as Docker, Container},
//     };
//     static DOCKER: OnceLock<Docker> = OnceLock::new();
//     static POSTGRES: OnceLock<Container<'static, Postgres>> = OnceLock::new();
//     #[tokio::test]
//     async fn test_execute_raw() -> Result<(), IntegrationOSError> {
//         let docker = DOCKER.get_or_init(Default::default);
//         let postgres = POSTGRES.get_or_init(|| docker.run(Postgres::default()));
//         let port = postgres.get_host_port_ipv4(5432);
//         println!("Connecting to postgres at port {port}");
//         let storage = StorageConfig::init_from_hashmap(&HashMap::from([
//             ("DATABASE_PORT".to_string(), port.to_string()),
//             ("STORAGE_CONFIG_TYPE".to_string(), "postgres".to_string()),
//         ]))
//         .expect("Failed to initialize storage config");
//         let postgres = PostgresStorage::new(&storage).await?;
//         let query = "SELECT 1".to_string();
//         let result = postgres.execute_raw(&query).await?;
//         let value = result.first().expect("Failed to get row");
//         println!("{value:?}");
//         assert_eq!(result.len(), 1);
//         assert_eq!(value.get("1"), Some(&Value::Number(Number::from(1))));
//         Ok(())
//     }
