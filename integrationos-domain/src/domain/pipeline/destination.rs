use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::prelude::connection::connection_model_definition::CrudAction;

#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "dummy", derive(fake::Dummy))]
pub enum Action {
    Passthrough {
        #[serde(with = "http_serde_ext_ios::method")]
        method: http::Method,
        #[cfg_attr(feature = "dummy", dummy(expr = "String::new().into()"))]
        path: Arc<str>,
    },
    Unified {
        #[cfg_attr(feature = "dummy", dummy(expr = "String::new().into()"))]
        name: Arc<str>,
        action: CrudAction,
        #[cfg_attr(feature = "dummy", dummy(default))]
        id: Option<Arc<str>>,
        #[serde(default)]
        passthrough: bool,
    },
}

impl Action {
    pub fn name(&self) -> &str {
        match self {
            Action::Passthrough { path, .. } => path,
            Action::Unified { name, .. } => name,
        }
    }

    pub fn action(&self) -> Option<&CrudAction> {
        match self {
            Action::Passthrough { .. } => None,
            Action::Unified { action, .. } => Some(action),
        }
    }

    pub fn passthrough(&self) -> bool {
        match self {
            Action::Passthrough { .. } => true,
            Action::Unified { passthrough, .. } => *passthrough,
        }
    }

    pub fn set_passthrough(mut self, value: bool) -> Self {
        match &mut self {
            Action::Passthrough { .. } => {}
            Action::Unified { passthrough, .. } => *passthrough = value,
        }

        self
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "dummy", derive(fake::Dummy))]
#[serde(rename_all = "camelCase")]
pub struct Destination {
    #[cfg_attr(feature = "dummy", dummy(expr = "String::new().into()"))]
    pub platform: Arc<str>,
    pub action: Action,
    #[cfg_attr(feature = "dummy", dummy(expr = "String::new().into()"))]
    pub connection_key: Arc<str>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_set_passthrough() {
        let action = Action::Unified {
            name: "test".to_string().into(),
            action: CrudAction::GetOne,
            id: None,
            passthrough: false,
        };

        assert!(!action.passthrough());

        let action = action.set_passthrough(true);

        assert!(action.passthrough());
    }

    #[test]
    fn test_destination_deserialize() {
        let action = r#"{
            "platform": "ios",
            "action": {
                "Unified": {
                    "name": "test",
                    "action": "getOne",
                    "id": null
                }
            },
            "connectionKey": "test"
        }"#;

        let destination: Destination = serde_json::from_str(action).unwrap();

        assert!(!destination.action.passthrough());
    }
}
