pub mod prefix;

use crate::{id::prefix::IdPrefix, IntegrationOSError, InternalError};
use base64ct::{Base64UrlUnpadded, Encoding};
use chrono::{DateTime, TimeZone, Utc};
use serde::{Deserialize, Serialize};
use std::{
    fmt::{Display, Formatter},
    str::FromStr,
};
use uuid::Uuid;

#[derive(Debug, Copy, Clone, Ord, PartialOrd, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "dummy", derive(fake::Dummy))]
#[serde(try_from = "String", into = "String")]
pub struct Id {
    prefix: IdPrefix,
    time: DateTime<Utc>,
    uuid: Uuid,
}

impl Id {
    pub fn new(prefix: IdPrefix, time: DateTime<Utc>) -> Self {
        Self {
            prefix,
            time,
            uuid: Uuid::new_v4(),
        }
    }

    pub fn now(prefix: IdPrefix) -> Self {
        Self {
            prefix,
            time: Utc::now(),
            uuid: Uuid::new_v4(),
        }
    }

    pub fn new_with_uuid(prefix: IdPrefix, time: DateTime<Utc>, uuid: Uuid) -> Self {
        Self { prefix, time, uuid }
    }

    pub fn test(prefix: IdPrefix) -> Self {
        Self {
            prefix,
            time: Utc
                .timestamp_opt(0, 0)
                .single()
                .expect("Failed to get UTC time"),
            uuid: Uuid::nil(),
        }
    }
}

impl Display for Id {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let Some(ts) = self.time.timestamp_nanos_opt() else {
            return write!(f, "Invalid Id");
        };
        let timestamp = Base64UrlUnpadded::encode_string(ts.to_be_bytes().as_slice());
        let random = Base64UrlUnpadded::encode_string(self.uuid.as_bytes());
        write!(f, "{}::{timestamp}::{random}", self.prefix)
    }
}

impl From<Id> for String {
    fn from(value: Id) -> String {
        value.to_string()
    }
}

impl TryFrom<String> for Id {
    type Error = IntegrationOSError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Id::from_str(value.as_str())
    }
}

impl FromStr for Id {
    type Err = IntegrationOSError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut parts = s.splitn(3, "::");
        let prefix = parts
            .next()
            .ok_or(InternalError::invalid_argument(
                &format!("Invalid ID: {}", s),
                None,
            ))?
            .try_into()?;

        let timestamp = parts.next().ok_or(InternalError::invalid_argument(
            &format!("Invalid ID: {}", s),
            None,
        ))?;
        let mut timestamp_buf = [0u8; 8];
        Base64UrlUnpadded::decode(timestamp.as_bytes(), &mut timestamp_buf)
            .map_err(|e| InternalError::invalid_argument(&format!("Invalid ID: {}", e), None))?;
        let timestamp = i64::from_be_bytes(timestamp_buf);
        let time = Utc.timestamp_nanos(timestamp);

        let uuid = parts.next().ok_or(InternalError::invalid_argument(
            &format!("Invalid ID: {}", s),
            None,
        ))?;
        let mut uuid_buf = [0u8; 16];
        Base64UrlUnpadded::decode(uuid, &mut uuid_buf)
            .map_err(|e| InternalError::invalid_argument(&format!("Invalid ID: {}", e), None))?;

        let uuid = Uuid::from_bytes(uuid_buf);

        Ok(Self { prefix, time, uuid })
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use once_cell::sync::Lazy;

    const ID_STR: &str = "evt::AAAAAAAAAAA::AAAAAAAAAAAAAAAAAAAAAA";
    static PARSED_ID: Lazy<Id> = Lazy::new(|| {
        Id::new_with_uuid(
            IdPrefix::Event,
            Utc.timestamp_opt(0, 0).single().unwrap(),
            Uuid::nil(),
        )
    });

    #[test]
    fn test_id_from_str() {
        let id = Id::from_str(ID_STR).unwrap();
        assert_eq!(id.prefix, PARSED_ID.prefix);
        assert_eq!(id.time, PARSED_ID.time);
        assert_eq!(id.uuid, PARSED_ID.uuid);
    }

    #[test]
    fn test_id_from_str_invalid() {
        assert!(Id::from_str("foo::AAAAAAAAAAA::AAAAAAAAAAAAAAAAAAAAAA").is_err());
        assert!(Id::from_str("evt::AAAAAAAAAAA::AAAAAAAAAAAAAAAAAAAAAAS").is_err());
        assert!(Id::from_str("evt::AAAAAAAAAAAS::AAAAAAAAAAAAAAAAAAAAAA").is_err());
        assert!(Id::from_str("evt::AAAAAAAAAAA::AAAAAAAAAAAAAAAAAAAAAA::").is_err());
    }

    #[test]
    fn test_id_display() {
        assert_eq!(format!("{}", *PARSED_ID), ID_STR);
    }

    #[test]
    fn test_id_event_key() {
        let id_str = "evt_k::AAAAAAAAAAA::AAAAAAAAAAAAAAAAAAAAAA";
        let id = Id::new_with_uuid(
            IdPrefix::EventKey,
            Utc.timestamp_opt(0, 0).single().unwrap(),
            Uuid::nil(),
        );
        assert_eq!(id, Id::from_str(id_str).unwrap());
        assert_eq!(id.to_string(), id_str);
    }

    #[test]
    fn test_id_serde() {
        let id = Id::new_with_uuid(
            IdPrefix::Event,
            Utc.timestamp_opt(0, 0).single().unwrap(),
            Uuid::nil(),
        );
        let id_str = serde_json::to_string(&id).unwrap();
        assert_eq!(id_str, format!("\"{}\"", ID_STR));
        assert_eq!(id, serde_json::from_str(&id_str).unwrap());
    }
}
