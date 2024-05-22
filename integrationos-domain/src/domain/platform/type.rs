use crate::Id;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
#[cfg_attr(feature = "dummy", derive(fake::Dummy))]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum PageType {
    SchemaUngenerated,
    Schema {
        #[serde(rename = "connectionModelSchemaId")]
        connection_model_schema_id: Id,
    },
    ActionUngenerated {
        #[serde(rename = "connectionModelSchemaId")]
        connection_model_schema_id: Id,
    },
    Action {
        #[serde(rename = "connectionModelDefinitionId")]
        connection_model_definition_id: Id,
        #[serde(rename = "connectionModelSchemaId")]
        connection_model_schema_id: Id,
    },
}
