use crate::{database::PostgresConfig, Id};
use serde::{Deserialize, Serialize};

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub struct DatabaseConnectionSecret {
    pub namespace: String,
    pub service_name: String,
    pub connection_id: Id,
    pub postgres_config: PostgresConfig,
}
