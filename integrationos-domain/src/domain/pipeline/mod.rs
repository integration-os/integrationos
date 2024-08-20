pub mod destination;
pub mod extractor;
pub mod middleware;
pub mod policies;
pub mod signature;
pub mod source;

use self::{
    destination::Destination, middleware::Middleware, signature::Signature, source::Source,
};
use super::{
    configuration::{environment::Environment, pipeline::PipelineConfig},
    shared::{ownership::Ownership, record_metadata::RecordMetadata},
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "dummy", derive(fake::Dummy))]
#[serde(rename_all = "camelCase")]
pub struct Pipeline {
    #[serde(rename = "_id")]
    pub id: String,
    pub environment: Environment,
    pub name: String,
    pub key: String,
    pub source: Source,
    pub destination: Destination,
    pub middleware: Vec<Middleware>,
    pub ownership: Ownership,
    pub signature: Signature,
    pub config: Option<PipelineConfig>,
    #[serde(flatten, default)]
    pub record_metadata: RecordMetadata,
}
