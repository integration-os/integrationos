pub mod page;
pub mod r#type;

use crate::{
    id::{prefix::IdPrefix, Id},
    {
        ownership::{Owners, SystemOwner},
        record_metadata::RecordMetadata,
    },
};
use bson::doc;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
#[cfg_attr(feature = "dummy", derive(fake::Dummy))]
#[serde(rename_all = "camelCase")]
pub struct PlatformData {
    #[serde(rename = "_id")]
    pub id: Id,
    pub connection_definition_id: Id,
    pub name: String,
    pub url: String,
    pub platform_version: String,

    #[serde(flatten, default)]
    pub record_metadata: RecordMetadata,
    pub ownership: Owners,
    pub analyzed: bool,
}

impl PlatformData {
    pub fn new(connection_definition_id: Id, name: String, url: String, version: String) -> Self {
        Self {
            id: Id::new(IdPrefix::Platform, chrono::Utc::now()),
            connection_definition_id,
            name,
            record_metadata: RecordMetadata::default(),
            ownership: Owners::System(SystemOwner {
                entity: "Event-Inc".to_string(),
                is_internal: true,
            }),
            url,
            platform_version: version,
            analyzed: false,
        }
    }
}
