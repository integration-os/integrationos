use super::{PipelineContext, Transaction};
use crate::{
    id::Id,
    prelude::{PipelineExt, PipelineStatus},
};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{fmt::Display, sync::Arc};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtractorContext {
    pub extractor_key: String,
    pub pipeline_key: String,
    pub event_key: Id,
    pub status: PipelineStatus,
    pub stage: Stage,
    #[serde(with = "chrono::serde::ts_milliseconds")]
    pub timestamp: DateTime<Utc>,
    r#type: Arc<str>,

    #[serde(flatten)]
    pub transaction: Option<Transaction>,
}

impl ExtractorContext {
    pub fn new(extractor_key: String, context: &PipelineContext) -> Self {
        Self {
            extractor_key,
            pipeline_key: context.pipeline_key.clone(),
            event_key: context.event_key,
            status: PipelineStatus::Succeeded,
            stage: Stage::New,
            timestamp: Utc::now(),
            r#type: "extractor".into(),
            transaction: None,
        }
    }

    pub fn is_dropped(&self) -> bool {
        matches!(self.status, PipelineStatus::Dropped { .. })
    }

    pub fn is_finished(&self) -> bool {
        matches!(self.stage, Stage::FinishedExtractor(..))
    }
}

#[async_trait]
impl PipelineExt for ExtractorContext {
    fn is_complete(&self) -> bool {
        self.is_dropped() || self.is_finished()
    }

    fn context_key(&self) -> &Id {
        &self.event_key
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Stage {
    New,
    FinishedExtractor(Value),
}

impl Display for Stage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::New => write!(f, "New"),
            Self::FinishedExtractor(v) => {
                write!(f, "FinishedExtractor({v})")
            }
        }
    }
}
