use crate::id::Id;
use downcast_rs::{impl_downcast, Downcast};
use serde::{Deserialize, Serialize};
use std::fmt::{Debug, Display};

pub trait PipelineExt: Downcast + Sync + Send + Debug {
    fn is_complete(&self) -> bool;
    fn context_key(&self) -> &Id;
}
impl_downcast!(PipelineExt);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PipelineStatus {
    Succeeded,
    Dropped { reason: String },
}

impl Display for PipelineStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Succeeded => write!(f, "Succeeded"),
            Self::Dropped { reason } => {
                write!(f, "Dropped {{ {reason} }}")
            }
        }
    }
}
