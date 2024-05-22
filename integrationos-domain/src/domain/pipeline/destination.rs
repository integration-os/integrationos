use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::prelude::connection::connection_model_definition::CrudAction;

#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "dummy", derive(fake::Dummy))]
pub enum Action {
    Passthrough {
        #[serde(with = "http_serde_ext::method")]
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
    },
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
