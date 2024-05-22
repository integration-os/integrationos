use crate::{IntegrationOSError, InternalError};
use serde::{Deserialize, Serialize};
use std::{
    fmt::{Display, Formatter},
    str::FromStr,
};

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Deserialize, Serialize, Hash)]
#[cfg_attr(feature = "dummy", derive(fake::Dummy))]
#[serde(rename_all = "camelCase")]
pub enum Environment {
    Test,
    Development,
    Live,
    Production,
}

impl TryFrom<&str> for Environment {
    type Error = IntegrationOSError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "test" => Ok(Environment::Test),
            "development" => Ok(Environment::Development),
            "live" => Ok(Environment::Live),
            "production" => Ok(Environment::Production),
            _ => Err(InternalError::configuration_error(
                &format!("Invalid environment: {}", value),
                None,
            )),
        }
    }
}

impl FromStr for Environment {
    type Err = IntegrationOSError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::try_from(s)
    }
}

impl Display for Environment {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let environment = match self {
            Environment::Test => "test",
            Environment::Development => "development",
            Environment::Production => "production",
            Environment::Live => "live",
        };
        write!(f, "{environment}")
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_environment_try_from() {
        assert_eq!(Environment::try_from("test").unwrap(), Environment::Test);
        assert_eq!(
            Environment::try_from("development").unwrap(),
            Environment::Development
        );
        assert_eq!(Environment::try_from("live").unwrap(), Environment::Live);
        assert!(Environment::try_from("invalid").is_err());
    }

    #[test]
    fn test_environment_display() {
        assert_eq!(format!("{}", Environment::Test), "test");
        assert_eq!(format!("{}", Environment::Development), "development");
        assert_eq!(format!("{}", Environment::Live), "live");
    }

    #[test]
    fn test_environment_from_str() {
        assert_eq!(Environment::from_str("test").unwrap(), Environment::Test);
        assert_eq!(
            Environment::from_str("development").unwrap(),
            Environment::Development
        );
        assert_eq!(Environment::from_str("live").unwrap(), Environment::Live);
        assert!(Environment::from_str("invalid").is_err());
    }
}
