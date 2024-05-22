use std::fmt::{Display, Formatter};

use bson::doc;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    id::{prefix::IdPrefix, Id},
    prelude::shared::record_metadata::RecordMetadata,
};

use super::JobStatus;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Stage<T: Serialize = Value> {
    #[serde(rename = "_id")]
    pub id: Id,
    pub job_id: Id,
    pub status: JobStatus,
    pub context: T,
    pub chat_history: Vec<Message>,
    #[serde(flatten, default)]
    pub record_metadata: RecordMetadata,
}

impl<T: Serialize> Stage<T> {
    pub fn new(
        job_id: Id,
        status: JobStatus,
        context: T,
        chat_history: Option<Vec<Message>>,
    ) -> Self {
        Stage {
            id: Id::now(IdPrefix::JobStage),
            job_id,
            status,
            context,
            chat_history: chat_history.unwrap_or_default(),
            record_metadata: Default::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Message {
    pub sender: Sender,
    pub message: String,
    #[serde(with = "chrono::serde::ts_milliseconds")]
    pub created_at: DateTime<Utc>,
}

impl Message {
    pub fn initial(message: String) -> Self {
        Self {
            sender: Sender::InitialPrompt,
            message,
            created_at: Utc::now(),
        }
    }

    pub fn human(message: String) -> Self {
        Self {
            sender: Sender::Human,
            message,
            created_at: Utc::now(),
        }
    }

    pub fn agent(message: String) -> Self {
        Self {
            sender: Sender::Agent,
            message,
            created_at: Utc::now(),
        }
    }

    pub fn system(message: String) -> Self {
        Self {
            sender: Sender::System,
            message,
            created_at: Utc::now(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub enum Sender {
    InitialPrompt,
    Human,
    Agent,
    System,
}

impl Display for Message {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self.sender {
            Sender::InitialPrompt => write!(f, "\n\nSystem: {}", self.message),
            Sender::Human => write!(f, "\n\nHuman: {}", self.message),
            Sender::Agent => write!(f, "\n\nAssistant: {}", self.message),
            Sender::System => Ok(()),
        }
    }
}
