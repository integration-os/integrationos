use crate::domain::postgres::PostgresStorage;
use async_trait::async_trait;
use futures::{StreamExt, TryStreamExt};
use integrationos_domain::{ApplicationError, IntegrationOSError};
use sqlx::postgres::PgRow;
use sqlx::{query, Column, PgPool, Row, ValueRef as PgValue};
use std::collections::HashMap;

const MAX_LIMIT: usize = 100;
// const MAX_SIZE: usize = 100;

#[async_trait]
pub trait Storage: Send + Sync {
    async fn execute_raw(
        &self,
        query: &'static str,
    ) -> Result<Vec<HashMap<String, Option<String>>>, IntegrationOSError>;
}

#[async_trait]
impl Storage for PostgresStorage {
    async fn execute_raw(
        &self,
        sql: &'static str,
    ) -> Result<Vec<HashMap<String, Option<String>>>, IntegrationOSError> {
        let rows = fetch_query(sql, &self.pool).await;

        let json_results = process_rows(rows)?;

        Ok(json_results)
    }
}

async fn fetch_query(sql: &'static str, pool: &PgPool) -> Vec<Result<PgRow, IntegrationOSError>> {
    query(sql)
        .fetch(pool)
        .take(MAX_LIMIT)
        .map_err(|e| {
            ApplicationError::bad_request(&format!("Failed to execute query: {}", e), None)
        })
        .collect::<Vec<Result<PgRow, IntegrationOSError>>>()
        .await
}

fn process_rows(
    rows: Vec<Result<PgRow, IntegrationOSError>>,
) -> Result<Vec<HashMap<String, Option<String>>>, IntegrationOSError> {
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
        .collect::<Result<Vec<HashMap<String, Option<String>>>, IntegrationOSError>>()
}

fn process_columns(row: PgRow) -> Result<HashMap<String, Option<String>>, IntegrationOSError> {
    row.columns()
        .iter()
        .try_fold(HashMap::new(), |mut acc, col| {
            let value = row.try_get_raw(col.ordinal()).map_err(|e| {
                ApplicationError::bad_request(&format!("Failed to get raw value: {}", e), None)
            })?;

            let value = if value.is_null() {
                None
            } else {
                value.as_str().map(|v| Some(v.to_string())).unwrap_or(None)
            };

            acc.insert(col.name().to_string(), value);

            Ok(acc)
        })
}
