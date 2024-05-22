//! AccessKey contains the data necessary to identify the source and destination of an event.
//! Every event request contains an EncryptedAccessKey, which together with a password
//! decrypts into an AccessKey.
//!
//! An access key is a string with the following format:
//! "{id or sk}_{live or test}_{version}_{encrypted data}"
//! This is represented by the EncryptedAccessKey struct.
//!
//! The encrypted data is binary encoded data with the following format:
//! "[content][iv (16 bytes)][hash (32 bytes)]"
//! This is represented by the EncryptedData struct.
//!
//! The decrypted content is a protobuf message representing the AccessKeyData struct.
//!
//! The event name path is either a json path or a string. If it begins with "_." it is the path to the event name
//! in an object of the form { _: { headers, query, body }}, where headers, query and body are json objects of the
//! different request parts. If it does not begin with "_." it is the event name itself.
//! The event name is extracted inside Server::get_event_name.
//!
//! Together with the event name, the topic of the event is constructed from the AccessKey.
//! The topic is a string with the following format:
//! "{event version}/{id}.{namespace}.{environment}.{event type}.{group}.{event name}"

pub mod access_key_data;
pub mod access_key_prefix;
pub mod encrypted_access_key;
pub mod encrypted_data;
pub mod event_type;

use self::{
    access_key_data::AccessKeyData, access_key_prefix::AccessKeyPrefix,
    encrypted_access_key::EncryptedAccessKey,
};
use crate::IntegrationOSError;
use base64ct::{Base64UrlUnpadded, Encoding};
use encrypted_data::{EncryptedData, IV_LENGTH, PASSWORD_LENGTH};
use std::str;

const EVENT_VERSION: &str = "v1";

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct AccessKey {
    pub prefix: AccessKeyPrefix,
    pub data: AccessKeyData,
}

impl AccessKey {
    pub fn get_topic(&self, event_name: &str) -> String {
        let id = &self.data.id;
        let namespace = &self.data.namespace;
        let environment = self.prefix.environment;
        let event_type = &self.data.event_type;
        let group = &self.data.group;
        format!("{EVENT_VERSION}/{id}.{namespace}.{environment}.{event_type}.{group}.{event_name}")
    }

    pub fn parse_str(
        access_key: &str,
        password: &[u8; PASSWORD_LENGTH],
    ) -> Result<Self, IntegrationOSError> {
        let access_key = EncryptedAccessKey::parse(access_key)?;
        AccessKey::parse(&access_key, password)
    }

    pub fn parse(
        access_key: &EncryptedAccessKey,
        password: &[u8; PASSWORD_LENGTH],
    ) -> Result<Self, IntegrationOSError> {
        let mut encrypted_content = access_key.get_encrypted_data()?;
        let decrypted_content = encrypted_content.verify_and_decrypt(password)?;
        let data = AccessKeyData::from_slice(decrypted_content)?;

        Ok(AccessKey {
            prefix: access_key.prefix,
            data,
        })
    }

    pub fn encode(
        &self,
        password: &[u8; PASSWORD_LENGTH],
        iv: &[u8; IV_LENGTH],
    ) -> Result<EncryptedAccessKey, IntegrationOSError> {
        let content = self.data.to_vec()?;
        let content = EncryptedData::encrypt(content, iv, password)?;
        let content = Base64UrlUnpadded::encode_string(&content);
        Ok(EncryptedAccessKey::new(self.prefix, content))
    }
}

#[cfg(test)]
mod tests {

    use tests::event_type::EventType;

    use crate::prelude::configuration::environment::Environment;

    use super::*;

    const VALID_KEY: &str = "id_live_1_Q71YUIZydcgSwJQNOUCHhaTMqmIvslIafF5LluORJfJKydMGELHtYe_ydtBIrVuomEnOZ4jfZQgtkqWxtG-s7vhbyir4kNjLyHKyDyh1SDubBMlhSI7Mq-M5RVtwnwFqZiOeUkIgHJFgcGQn0Plb1AkAAAAAAAAAAAAAAAAAAAAAAMwWY_9_oDOV75noniBViOVmVPUQqzcW8G3P8nuUD6Q";
    const VALID_PASSWORD: &[u8; PASSWORD_LENGTH] = b"32KFFT_i4UpkJmyPwY2TGzgHpxfXs7zS";

    #[test]
    fn test_access_key_round_trip() {
        let data = AccessKey {
            prefix: AccessKeyPrefix {
                environment: Environment::Live,
                event_type: EventType::Id,
                version: 1,
            },
            data: AccessKeyData {
                id: "build-2e76c839f5fd419db6b34682f4cdff1e".to_owned(),
                namespace: "default".to_owned(),
                event_type: "webhook".to_owned(),
                group: "my-webhook".to_owned(),
                event_path: "event.received".to_owned(),
                event_object_id_path: Some("foo.bar".to_owned()),
                timestamp_path: Some("foo.bar".to_owned()),
                parent_access_key: Some("foo.bar".to_owned()),
            },
        };

        let encrypted = data.encode(VALID_PASSWORD, &[0u8; IV_LENGTH]).unwrap();
        let decrypted = AccessKey::parse(&encrypted, VALID_PASSWORD).unwrap();
        assert_eq!(data, decrypted);
    }

    #[test]
    fn test_parse_valid_access_key() {
        let data = AccessKey {
            prefix: AccessKeyPrefix {
                environment: Environment::Live,
                event_type: EventType::Id,
                version: 1,
            },
            data: AccessKeyData {
                id: "build-2e76c839f5fd419db6b34682f4cdff1e".to_owned(),
                namespace: "default".to_owned(),
                event_type: "webhook".to_owned(),
                group: "my-webhook".to_owned(),
                event_path: "event.received".to_owned(),
                event_object_id_path: Some("foo.bar".to_owned()),
                timestamp_path: Some("foo.bar".to_owned()),
                parent_access_key: Some("foo.bar".to_owned()),
            },
        };

        let decrypt = AccessKey::parse_str(VALID_KEY, VALID_PASSWORD).unwrap();
        assert_eq!(data, decrypt);
    }

    #[test]
    fn test_invalid_access_key() {
        let key = "id_live_1_anJIdjhNUlMxcWRYcU1FT3FnWHJkSFE3Nlh1eEp1Y0I0UTJRdFBIR1BnU0V6ZTg5MUE0WTVseUpSVGQ3VkNQaEV0bmVicE1oMUR4WU4xYTRpczltLXBFZWE5Y05ka0ctaWxnODBPa24tU3A4ZFR5T3J1TS1GaU9PQjdhSUJDbmh6ZHp4RWpDRWJ5WUxTSVR2ZlNKSlNSU0ZvUSVaSDhUVlNXcHdnLTY4VDltcEpBMnV3JW1UV1AyVkllT3hiTEZYZGtLYXBvLVJRdXVNVEtwc1JJUFNoTTNJc21uRmN";
        let res = AccessKey::parse_str(key, VALID_PASSWORD);
        assert!(res.is_err());
    }

    #[test]
    fn test_invalid_password() {
        let password = b"vOVH6sdmpNWjRRIqCc7rdxs01lxHzfr3";
        let res = AccessKey::parse_str(VALID_KEY, password);
        assert!(res.is_err());
    }

    #[test]
    fn test_get_topic() {
        let data = AccessKey {
            prefix: AccessKeyPrefix {
                environment: Environment::Live,
                event_type: EventType::Id,
                version: 1,
            },
            data: AccessKeyData {
                id: "build-2e76c839f5fd419db6b34682f4cdff1e".to_owned(),
                namespace: "default".to_owned(),
                event_type: "webhook".to_owned(),
                group: "my-webhook".to_owned(),
                event_path: "event.received".to_owned(),
                event_object_id_path: None,
                timestamp_path: None,
                parent_access_key: None,
            },
        };

        let name = "event.received";
        let topic = data.get_topic(name);
        assert_eq!(
            topic,
            "v1/build-2e76c839f5fd419db6b34682f4cdff1e.default.live.webhook.my-webhook.event.received"
        );
    }
}
