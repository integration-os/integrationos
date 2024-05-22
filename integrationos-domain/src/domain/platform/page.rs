use crate::{
    hashed_secret::HashedSecret,
    id::{prefix::IdPrefix, Id},
    ownership::{Owners, SystemOwner},
    r#type::PageType,
    record_metadata::RecordMetadata,
};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
#[cfg_attr(feature = "dummy", derive(fake::Dummy))]
#[serde(rename_all = "camelCase")]
pub struct PlatformPage {
    #[serde(rename = "_id")]
    pub id: Id,
    pub platform_id: Id,
    pub platform_name: String,
    pub connection_definition_id: Id,
    #[serde(flatten)]
    pub r#type: PageType,
    pub url: String,
    pub model_name: String,
    pub content: String,
    pub hashed_content: String,
    #[serde(flatten, default)]
    pub record_metadata: RecordMetadata,
    pub ownership: Owners,
    pub analyzed: bool,
    pub job_started: bool,
}

impl PlatformPage {
    pub fn new(
        platform_id: Id,
        connection_definition_id: Id,
        platform_name: String,
        r#type: PageType,
        url: String,
        model_name: String,
        content: String,
    ) -> Result<Self> {
        let hash_value = json!({
            "platform_id": platform_id,
            "platform_name": platform_name,
            "model_name": model_name,
            "page_type": r#type,
            "content": content
        });

        let hashed = HashedSecret::try_from(hash_value)?;

        Ok(Self {
            id: Id::new(IdPrefix::PlatformPage, chrono::Utc::now()),
            connection_definition_id,
            platform_id,
            platform_name,
            r#type,
            url,
            model_name,
            content,
            hashed_content: hashed.into_inner(),
            record_metadata: RecordMetadata::default(),
            ownership: Owners::System(SystemOwner {
                entity: "Event-Inc".to_string(),
                is_internal: true,
            }),
            analyzed: false,
            job_started: false,
        })
    }
}
