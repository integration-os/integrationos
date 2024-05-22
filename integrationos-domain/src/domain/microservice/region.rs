use crate::{IntegrationOSError, InternalError};
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};

const US_EAST_1: &str = "us-east1";

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum Region {
    UsEast1,
}

impl Display for Region {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let region = match self {
            Region::UsEast1 => US_EAST_1,
        };

        write!(f, "{region}")
    }
}

impl TryFrom<&str> for Region {
    type Error = IntegrationOSError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            US_EAST_1 => Ok(Region::UsEast1),
            _ => Err(InternalError::invalid_argument("Invalid region", None)),
        }
    }
}
