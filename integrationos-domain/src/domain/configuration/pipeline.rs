use crate::policies::{Policies, RetryPolicy};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq)]
#[cfg_attr(feature = "dummy", derive(fake::Dummy))]
#[serde(rename_all = "camelCase")]
pub struct PipelineConfig {
    pub policies: Policies,
    pub start_to_close_timeout: String,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            policies: Policies {
                retry: RetryPolicy {
                    maximum_attempts: 3,
                    initial_interval: "1 second".to_owned(),
                },
            },
            start_to_close_timeout: "10 seconds".to_owned(),
        }
    }
}
