pub mod stage;
use super::shared::record_metadata::RecordMetadata;
use crate::id::{prefix::IdPrefix, Id};
use bson::doc;
use serde::{Deserialize, Serialize};
use strum::Display;

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Job {
    #[serde(rename = "_id")]
    pub id: Id,
    pub name: String,
    pub job_type: JobType,
    pub status: JobStatus,
    pub stage: Id,
    pub parent: Option<Id>,
    #[serde(flatten, default)]
    pub record_metadata: RecordMetadata,
}

impl Default for Job {
    fn default() -> Self {
        Self {
            id: Id::now(IdPrefix::Job),
            name: Default::default(),
            job_type: JobType::CommonModelChain,
            status: JobStatus::InProgress,
            stage: Id::now(IdPrefix::JobStage),
            parent: Default::default(),
            record_metadata: Default::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Eq, PartialEq)]
pub enum JobType {
    CommonModelChain,
    DiscoveryChain,
    MapDefinitionChain,
    MapIndividualModelChain,
    MapJavascriptChain,
    MappingChain,
    PlatformAnalyzer,
    PlatformGenerator,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Display)]
pub enum JobStatus {
    /// Job is in progress, this is the default state
    InProgress,
    /// Job is waiting for approval from a user in the chat
    ApprovalRequired,
    /// Job is waiting for a user to join the chat to resolve an issue
    ChatRequired,
    /// Job is complete, can also be used to skip a stage in the chain
    Completed,
    /// Job is canceled
    Canceled,
    /// Job has failed
    Failed,
}
