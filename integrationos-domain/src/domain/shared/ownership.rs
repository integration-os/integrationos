use std::sync::Arc;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Eq, PartialEq, Hash, Deserialize, Serialize)]
#[cfg_attr(feature = "dummy", derive(fake::Dummy))]
#[serde(rename_all = "camelCase")]
pub struct Ownership {
    #[serde(rename = "buildableId")]
    #[cfg_attr(feature = "dummy", dummy(expr = "String::new().into()"))]
    pub id: Arc<str>,
    pub client_id: String,
    pub organization_id: Option<String>,
    pub project_id: Option<String>,
    pub user_id: Option<String>,
}

impl Default for Ownership {
    fn default() -> Self {
        Self {
            id: String::new().into(),
            client_id: Default::default(),
            organization_id: Default::default(),
            project_id: Default::default(),
            user_id: Default::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
#[cfg_attr(feature = "dummy", derive(fake::Dummy))]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum Owners {
    User(Ownership),
    System(SystemOwner),
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
#[cfg_attr(feature = "dummy", derive(fake::Dummy))]
#[serde(rename_all = "camelCase")]
pub struct SystemOwner {
    pub entity: String,
    pub is_internal: bool,
}

impl Ownership {
    pub fn new(id: String) -> Ownership {
        Self {
            id: id.clone().into(),
            client_id: id.clone(),
            ..Default::default()
        }
    }
}
