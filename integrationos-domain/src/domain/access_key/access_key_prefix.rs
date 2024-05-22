use super::event_type::EventType;
use crate::prelude::configuration::environment::Environment;
use std::fmt::{Display, Formatter};

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct AccessKeyPrefix {
    pub environment: Environment,
    pub event_type: EventType,
    pub version: u32,
}

impl Display for AccessKeyPrefix {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}_{}_{}",
            self.event_type, self.environment, self.version
        )
    }
}

impl AccessKeyPrefix {
    pub fn new(environment: Environment, event_type: EventType, version: u32) -> Self {
        Self {
            environment,
            event_type,
            version,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_access_key_prefix_display() {
        assert_eq!(
            format!(
                "{}",
                AccessKeyPrefix {
                    environment: Environment::Test,
                    event_type: EventType::Id,
                    version: 1
                }
            ),
            "id_test_1"
        );
        assert_eq!(
            format!(
                "{}",
                AccessKeyPrefix {
                    environment: Environment::Live,
                    event_type: EventType::SecretKey,
                    version: 2
                }
            ),
            "sk_live_2"
        );
    }

    #[test]
    fn test_access_key_prefix_new() {
        assert_eq!(
            AccessKeyPrefix::new(Environment::Test, EventType::Id, 1),
            AccessKeyPrefix {
                environment: Environment::Test,
                event_type: EventType::Id,
                version: 1,
            }
        );
        assert_eq!(
            AccessKeyPrefix::new(Environment::Live, EventType::SecretKey, 2),
            AccessKeyPrefix {
                environment: Environment::Live,
                event_type: EventType::SecretKey,
                version: 2,
            }
        );
    }
}
