use crate::Id;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DatabaseConnectionLost {
    pub connection_id: Id,
    pub reason: Option<String>,
    pub schedule_on: Option<i64>,
}

impl DatabaseConnectionLost {
    pub fn as_event(&self) -> Value {
        json!({
            "type": "DatabaseConnectionLost",
            "connectionId": self.connection_id,
            "reason": self.reason.clone(),
            "scheduleOn": self.schedule_on,
        })
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ConnectionLostReason {
    pub reason: String,
}
