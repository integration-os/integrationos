use crate::{
    id::{prefix::IdPrefix, Id},
    prelude::{
        configuration::environment::Environment,
        event::Event,
        shared::{ownership::Ownership, record_metadata::RecordMetadata},
    },
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Transaction {
    #[serde(rename = "_id")]
    pub id: Id,
    pub tx_key: String,
    pub input: String,
    pub output: String,
    pub txn: String,
    pub state: String,
    pub environment: Environment,
    #[serde(with = "chrono::serde::ts_milliseconds")]
    pub started_at: DateTime<Utc>,
    pub ownership: Ownership,
    pub event_id: Id,
    #[serde(flatten, default)]
    pub record_metadata: RecordMetadata,
}

impl Transaction {
    pub fn new(event: &Event, key: String, input: String, output: String, state: String) -> Self {
        let ts = Utc::now();
        let id = Id::new(IdPrefix::Transaction, ts);
        Transaction {
            id,
            tx_key: key,
            input,
            output,
            txn: Uuid::new_v4().simple().to_string(),
            environment: event.environment,
            state,
            started_at: ts,
            ownership: event.ownership.clone(),
            event_id: event.id,
            record_metadata: Default::default(),
        }
    }

    pub fn completed(event: &Event, key: String, input: String, output: String) -> Self {
        Self::new(event, key, input, output, "completed".to_owned())
    }

    pub fn failed(event: &Event, key: String, input: String, output: String) -> Self {
        Self::new(event, key, input, output, "failed".to_owned())
    }

    pub fn panicked(event: &Event, key: String, input: String, output: String) -> Self {
        Self::new(event, key, input, output, "panicked".to_owned())
    }

    pub fn throttled(event: &Event, key: String, input: String, output: String) -> Self {
        Self::new(event, key, input, output, "throttled".to_owned())
    }
}
