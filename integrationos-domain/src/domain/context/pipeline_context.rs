use super::{extractor_context::ExtractorContext, root_context::RootContext, Transaction};
use crate::{
    id::Id,
    prelude::{PipelineExt, PipelineStatus},
};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{collections::HashMap, fmt::Display, sync::Arc};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PipelineContext {
    pub pipeline_key: String,
    pub event_key: Id,
    pub status: PipelineStatus,
    pub stage: PipelineStage,
    #[serde(with = "chrono::serde::ts_milliseconds")]
    pub timestamp: DateTime<Utc>,
    r#type: Arc<str>,

    #[serde(flatten)]
    pub transaction: Option<Transaction>,
}

impl PipelineContext {
    pub fn new(pipeline_key: String, context: &RootContext) -> Self {
        Self {
            pipeline_key,
            event_key: context.event_key,
            status: PipelineStatus::Succeeded,
            stage: PipelineStage::New,
            timestamp: Utc::now(),
            r#type: "pipeline".into(),
            transaction: None,
        }
    }

    pub fn is_dropped(&self) -> bool {
        matches!(self.status, PipelineStatus::Dropped { .. })
    }

    pub fn is_finished(&self) -> bool {
        matches!(self.stage, PipelineStage::FinishedPipeline)
    }
}

#[async_trait]
impl PipelineExt for PipelineContext {
    fn is_complete(&self) -> bool {
        self.is_dropped() || self.is_finished()
    }

    fn context_key(&self) -> &Id {
        &self.event_key
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PipelineStage {
    New,
    ExecutingExtractors(HashMap<String, ExtractorContext>),
    ExecutedExtractors(HashMap<String, Value>),
    ExecutedTransformer(Option<Value>),
    FinishedPipeline,
}

impl Display for PipelineStage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::New => write!(f, "New"),
            Self::ExecutingExtractors(e) => {
                write!(f, "ExecutingExtractors(")?;
                for (s, e) in e.iter() {
                    write!(f, "{{{s} => {}: {}}}", e.stage, e.status)?;
                }
                write!(f, ")")
            }
            Self::ExecutedExtractors(m) => {
                write!(f, "ExecutedExtractors(")?;
                for (s, v) in m.iter() {
                    write!(f, "{{{s} => {v}}}")?;
                }
                write!(f, ")")
            }
            Self::ExecutedTransformer(Some(v)) => {
                write!(f, "ExecutedTransformer({v})")
            }
            Self::ExecutedTransformer(None) => {
                write!(f, "ExecutedTransformer()")
            }
            Self::FinishedPipeline => write!(f, "FinishedPipeline"),
        }
    }
}
