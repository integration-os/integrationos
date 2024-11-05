use integrationos_domain::{record_metadata::RecordMetadata, Id};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Deduplication {
    #[serde(rename = "_id")]
    pub entity_id: Id,
    #[serde(flatten)]
    pub metadata: RecordMetadata,
}
