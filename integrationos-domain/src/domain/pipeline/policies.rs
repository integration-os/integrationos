use crate::{IntegrationOSError, InternalError};
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq)]
#[cfg_attr(feature = "dummy", derive(fake::Dummy))]
pub struct Policies {
    pub retry: RetryPolicy,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "dummy", derive(fake::Dummy))]
#[serde(rename_all = "camelCase")]
pub struct RetryPolicy {
    #[cfg_attr(feature = "dummy", dummy(faker = "0..10"))]
    pub maximum_attempts: u64,
    pub initial_interval: String,
}

impl RetryPolicy {
    pub fn get_interval(&self) -> Result<Duration, IntegrationOSError> {
        let mut parts = self.initial_interval.split(' ');
        let num = parts
            .next()
            .ok_or(InternalError::configuration_error(
                "No number in retry policy interval",
                None,
            ))?
            .parse()
            .map_err(|e| {
                InternalError::configuration_error(
                    &format!("Invalid retry policy interval number: {}", e),
                    None,
                )
            })?;
        let amount = parts.next().ok_or(InternalError::configuration_error(
            "No amount in retry policy interval",
            None,
        ))?;
        match amount {
            "seconds" | "second" => Ok(Duration::from_secs(num)),
            "minute" | "minutes" => Ok(Duration::from_secs(num * 60)),
            x => Err(InternalError::configuration_error(
                &format!("Invalid retry policy interval amount: {}", x),
                None,
            )),
        }
    }
}
