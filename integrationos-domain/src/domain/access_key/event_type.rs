use crate::{IntegrationOSError, InternalError};
use std::fmt::{Display, Formatter};

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum EventType {
    Id,
    SecretKey,
}

impl TryFrom<&str> for EventType {
    type Error = IntegrationOSError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "id" => Ok(EventType::Id),
            "sk" => Ok(EventType::SecretKey),
            val => Err(InternalError::invalid_argument(
                &format!("Invalid event type: {}", val),
                None,
            )),
        }
    }
}

impl Display for EventType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let event_type = match self {
            EventType::Id => "id",
            EventType::SecretKey => "sk",
        };
        write!(f, "{}", event_type)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_event_type_try_from() {
        assert_eq!(EventType::try_from("id").unwrap(), EventType::Id);
        assert_eq!(EventType::try_from("sk").unwrap(), EventType::SecretKey);
        assert!(EventType::try_from("invalid").is_err());
    }

    #[test]
    fn test_event_type_display() {
        assert_eq!(format!("{}", EventType::Id), "id");
        assert_eq!(format!("{}", EventType::SecretKey), "sk");
    }
}
