use crate::domain::postgres::serialize_pgvalueref;
use crate::domain::postgres::PostgresStorage;
use async_trait::async_trait;
use futures::{StreamExt, TryStreamExt};
use integrationos_domain::{ApplicationError, IntegrationOSError};
use serde_json::Value;
use sqlx::postgres::PgRow;
use sqlx::{query, Column, PgPool, Row};
use std::collections::HashMap;

const MAX_LIMIT: usize = 100;
// const MAX_SIZE: usize = 100;

#[async_trait]
pub trait Storage: Send + Sync {
    async fn execute_raw(
        &self,
        query: &str,
    ) -> Result<Vec<HashMap<String, Value>>, IntegrationOSError>;

    async fn probe(&self) -> Result<bool, IntegrationOSError>;
}

#[async_trait]
impl Storage for PostgresStorage {
    async fn execute_raw(
        &self,
        sql: &str,
    ) -> Result<Vec<HashMap<String, Value>>, IntegrationOSError> {
        let rows = fetch_query(sql, &self.pool).await;

        let json_results = process_rows(rows)?;

        Ok(json_results)
    }

    async fn probe(&self) -> Result<bool, IntegrationOSError> {
        self.execute_raw("SELECT 1").await.map(|_| true)
    }
}

async fn fetch_query(sql: &str, pool: &PgPool) -> Vec<Result<PgRow, IntegrationOSError>> {
    query(sql)
        .fetch(pool)
        .take(MAX_LIMIT)
        .map_err(|e| {
            ApplicationError::bad_request(&format!("Failed to execute query: {}", e), None)
        })
        .collect::<Vec<Result<PgRow, IntegrationOSError>>>()
        .await
}

// PostgresStorage

fn process_rows(
    rows: Vec<Result<PgRow, IntegrationOSError>>,
) -> Result<Vec<HashMap<String, Value>>, IntegrationOSError> {
    rows.into_iter()
        .map(|result| {
            result.and_then(|row| {
                process_columns(row).map_err(|e| {
                    ApplicationError::bad_request(
                        &format!("Failed to convert to JSON: {}", e),
                        None,
                    )
                })
            })
        })
        .collect::<Result<Vec<HashMap<String, Value>>, IntegrationOSError>>()
}

fn process_columns(row: PgRow) -> Result<HashMap<String, Value>, IntegrationOSError> {
    row.columns()
        .iter()
        .try_fold(HashMap::new(), |mut acc, col| {
            let value = row.try_get_raw(col.ordinal()).map_err(|e| {
                ApplicationError::bad_request(&format!("Failed to get raw value: {}", e), None)
            })?;

            let mut buffer = Vec::new();
            let mut json_serializer = serde_json::Serializer::new(&mut buffer);

            // Serialize the value
            serialize_pgvalueref(&value, &mut json_serializer).map_err(|e| {
                ApplicationError::bad_request(&format!("Failed to serialize value: {}", e), None)
            })?;

            // Convert buffer to String
            // This assumes serialize_pgvalueref returns a valid JSON-like format.
            let serialized: Value = serde_json::from_slice(&buffer).map_err(|e| {
                ApplicationError::bad_request(&format!("Failed to serialize value: {}", e), None)
            })?;

            acc.insert(col.name().to_string(), serialized);

            Ok(acc)
        })
}
