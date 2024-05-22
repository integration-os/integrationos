use crate::{
    id::{prefix::IdPrefix, Id},
    prelude::{schema::json_schema::JsonSchema, shared::record_metadata::RecordMetadata},
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
#[cfg_attr(feature = "dummy", derive(fake::Dummy))]
#[serde(rename_all = "camelCase")]
pub struct ConnectionModelSchema {
    #[serde(rename = "_id")]
    pub id: Id,
    pub platform_id: Id,
    pub platform_page_id: Id,
    pub connection_platform: String,
    pub connection_definition_id: Id,
    pub platform_version: String,
    #[serde(default)]
    pub key: String,
    pub model_name: String,
    pub sample: Value,
    pub schema: JsonSchema,
    pub paths: Option<SchemaPaths>,
    #[cfg_attr(feature = "dummy", dummy(default))]
    pub mapping: Option<Mappings>,
    #[serde(flatten, default)]
    pub record_metadata: RecordMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
#[cfg_attr(feature = "dummy", derive(fake::Dummy))]
#[serde(rename_all = "camelCase")]
pub struct SchemaPaths {
    pub id: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
#[cfg_attr(feature = "dummy", derive(fake::Dummy))]
#[serde(rename_all = "camelCase")]
pub struct Mappings {
    pub from_common_model: String,
    pub to_common_model: String,
    pub common_model_name: String,
    pub common_model_id: Id,
    pub unmapped_fields: JsonSchema,
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
#[cfg_attr(feature = "dummy", derive(fake::Dummy))]
#[serde(rename_all = "camelCase")]
pub struct PublicMappings {
    pub common_model_name: String,
}

impl From<Mappings> for PublicMappings {
    fn from(mappings: Mappings) -> Self {
        PublicMappings {
            common_model_name: mappings.common_model_name,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
#[cfg_attr(feature = "dummy", derive(fake::Dummy))]
#[serde(rename_all = "camelCase")]
pub struct PublicConnectionModelSchema {
    #[serde(rename = "_id")]
    pub id: Id,
    pub connection_platform: String,
    pub connection_definition_id: Id,
    pub platform_version: String,
    pub model_name: String,
    pub mapping: PublicMappings,
    #[serde(flatten, default)]
    pub record_metadata: RecordMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionModelSchemaBuilder {
    pub platform_id: Id,
    pub platform_page_id: Id,
    pub connection_platform: String,
    pub connection_definition_id: Id,
    pub platform_version: String,
    pub model_name: String,
    pub sample: Value,
    pub schema: JsonSchema,
    pub paths: Option<SchemaPaths>,
    pub mapping: Option<Mappings>,
}

impl ConnectionModelSchema {
    pub fn new(input: ConnectionModelSchemaBuilder) -> Self {
        let key = format!(
            "api::{}::{}::{}",
            input.connection_platform, input.platform_version, input.model_name
        )
        .to_lowercase();

        Self {
            id: Id::new(IdPrefix::ConnectionModelSchema, chrono::Utc::now()),
            platform_id: input.platform_id,
            platform_page_id: input.platform_page_id,
            connection_platform: input.connection_platform,
            connection_definition_id: input.connection_definition_id,
            platform_version: input.platform_version,
            key,
            model_name: input.model_name,
            sample: input.sample,
            schema: input.schema,
            paths: input.paths,
            mapping: input.mapping,
            record_metadata: RecordMetadata::default(),
        }
    }
}
