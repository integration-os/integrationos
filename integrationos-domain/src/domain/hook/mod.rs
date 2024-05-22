use serde::{Deserialize, Serialize};

use crate::api_model_config::{ApiModelConfig, Function};

/// The hook struct models a hook in the api model.
///
/// A hook is a function that is called before or after a request is processed.
/// It contains an api model configuration which is used in case the hook requires
/// to make a request to the api.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "dummy", derive(fake::Dummy))]
pub struct Hook {
    pub before: Function,
    pub after: Function,
    pub configuration: ApiModelConfig,
}
