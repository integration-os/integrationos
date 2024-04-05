use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Deserialize, Serialize)]
pub struct EventRequest {
    pub event: String,
    pub payload: Value,
}
