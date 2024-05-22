use super::{api_model_config::AuthMethod, ConnectionType};
use crate::id::{prefix::IdPrefix, Id};
use crate::prelude::shared::{record_metadata::RecordMetadata, settings::Settings};
use serde::{Deserialize, Serialize};
use strum::{self, AsRefStr, Display};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "dummy", derive(fake::Dummy))]
#[serde(rename_all = "camelCase")]
pub struct ConnectionDefinition {
    #[serde(rename = "_id")]
    pub id: Id,
    pub platform_version: String,
    pub platform: String,
    #[serde(default)]
    pub status: ConnectionStatus,
    #[serde(default)]
    pub key: String,
    pub r#type: ConnectionDefinitionType,
    pub name: String,
    pub auth_secrets: Vec<AuthSecret>,
    pub auth_method: Option<AuthMethod>,
    pub frontend: Frontend,
    pub paths: Paths,
    pub settings: Settings,
    pub hidden: bool,
    pub test_connection: Option<Id>,
    #[serde(flatten, default)]
    pub record_metadata: RecordMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublicConnectionDetails {
    pub platform: String,
    pub models: Vec<ModelFeatures>,
    pub caveats: Vec<Caveat>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelFeatures {
    pub name: String,
    pub pagination: bool,
    pub filtration: bool,
    pub sorting: bool,
}

#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq)]
#[cfg_attr(feature = "dummy", derive(fake::Dummy))]
pub enum ConnectionStatus {
    NotAvailable,
    #[default]
    Beta,
    Alpha,
    GenerallyAvailable,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "dummy", derive(fake::Dummy))]
pub struct Caveat {
    pub connection_model_definition_id: Option<String>,
    pub comments: Vec<String>,
}

impl ConnectionDefinition {
    pub fn new(
        name: String,
        description: String,
        platform: String,
        platform_version: String,
        category: String,
        image: String,
        tags: Vec<String>,
    ) -> Self {
        let key = format!("api::{}::{}", platform, platform_version);

        Self {
            id: Id::now(IdPrefix::ConnectionDefinition),
            platform_version,
            platform: platform.clone(),
            r#type: ConnectionDefinitionType::Api,
            status: ConnectionStatus::Beta,
            name: name.clone(),
            key,
            frontend: Frontend {
                spec: Spec {
                    title: name.clone(),
                    description: description.clone(),
                    platform,
                    category,
                    image,
                    tags,
                    helper_link: None,
                },
                connection_form: ConnectionForm {
                    name,
                    description,
                    form_data: vec![],
                },
            },
            test_connection: None,
            auth_secrets: vec![],
            auth_method: None,
            paths: Paths {
                id: None,
                event: None,
                payload: None,
                timestamp: None,
                secret: None,
                signature: None,
                cursor: None,
            },
            settings: Settings {
                parse_webhook_body: false,
                show_secret: false,
                allow_custom_events: false,
                oauth: false,
            },
            hidden: true,
            record_metadata: RecordMetadata::default(),
        }
    }

    #[deprecated(since = "4.0.0", note = "Use `ConnectionType` instead")]
    pub fn to_connection_type(&self) -> super::ConnectionType {
        match self.r#type {
            ConnectionDefinitionType::Api => ConnectionType::Api {},
            ConnectionDefinitionType::DatabaseSql => ConnectionType::DatabaseSql {},
            ConnectionDefinitionType::DatabaseNoSql => ConnectionType::DatabaseNoSql,
            ConnectionDefinitionType::FileSystem => ConnectionType::FileSystem,
            ConnectionDefinitionType::Stream => ConnectionType::Stream,
            ConnectionDefinitionType::Custom => ConnectionType::Custom,
        }
    }

    pub fn set_oauth(&mut self, oauth: bool) {
        self.settings.oauth = oauth;
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "dummy", derive(fake::Dummy))]
#[serde(rename_all = "camelCase")]
pub struct AuthSecret {
    pub name: String,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, Display, AsRefStr)]
#[cfg_attr(feature = "dummy", derive(fake::Dummy))]
#[serde(rename_all = "lowercase", rename = "connectionType")]
#[strum(serialize_all = "lowercase")]
pub enum ConnectionDefinitionType {
    Api,
    DatabaseSql,
    DatabaseNoSql,
    FileSystem,
    Stream,
    Custom,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "dummy", derive(fake::Dummy))]
#[serde(rename_all = "camelCase")]
pub struct Frontend {
    pub spec: Spec,
    pub connection_form: ConnectionForm,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "dummy", derive(fake::Dummy))]
#[serde(rename_all = "camelCase")]
pub struct Spec {
    pub title: String,
    pub description: String,
    pub platform: String,
    pub category: String,
    pub image: String,
    pub tags: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub helper_link: Option<String>,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "dummy", derive(fake::Dummy))]
#[serde(rename_all = "camelCase")]
pub struct ConnectionForm {
    pub name: String,
    pub description: String,
    pub form_data: Vec<FormDataItem>,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "dummy", derive(fake::Dummy))]
#[serde(rename_all = "camelCase")]
pub struct FormDataItem {
    pub name: String,
    pub r#type: String,
    pub label: String,
    pub placeholder: String,
}

#[derive(Debug, Default, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "dummy", derive(fake::Dummy))]
#[serde(rename_all = "camelCase")]
pub struct Paths {
    pub id: Option<String>,
    pub event: Option<String>,
    pub payload: Option<String>,
    pub timestamp: Option<String>,
    pub secret: Option<String>,
    pub signature: Option<String>,
    pub cursor: Option<String>,
}
