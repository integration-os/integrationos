pub mod duplicates;
pub mod emitted_events;
pub mod event_access;
pub mod event_response;
pub mod event_state;
pub mod event_with_context;
pub mod hashes;

use chrono::{DateTime, SubsecRound, Utc};
use http::HeaderMap;
use serde::{Deserialize, Serialize};

use crate::id::{prefix::IdPrefix, Id};

use self::{
    duplicates::Duplicates,
    event_state::EventState,
    hashes::{HashValue, Hashes},
};

use super::{
    access_key::{encrypted_access_key::EncryptedAccessKey, AccessKey},
    configuration::environment::Environment,
    shared::{ownership::Ownership, record_metadata::RecordMetadata},
};

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[cfg_attr(feature = "dummy", derive(fake::Dummy))]
#[serde(rename_all = "camelCase")]
pub struct Event {
    #[serde(rename = "_id")]
    pub id: Id,
    pub key: Id,
    pub name: String,
    pub r#type: String,
    pub group: String,
    pub access_key: String,
    pub topic: String,
    pub environment: Environment,
    pub body: String,
    #[serde(with = "http_serde_ext_ios::header_map")]
    pub headers: HeaderMap,
    #[serde(with = "chrono::serde::ts_milliseconds")]
    pub arrived_at: DateTime<Utc>,
    pub arrived_date: DateTime<Utc>,
    pub state: EventState,
    pub ownership: Ownership,
    pub hashes: [HashValue; 3],
    pub payload_byte_length: usize,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub duplicates: Option<Duplicates>,
    #[serde(flatten, default)]
    pub record_metadata: RecordMetadata,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[cfg_attr(feature = "dummy", derive(fake::Dummy))]
#[serde(rename_all = "camelCase")]
pub struct PublicEvent {
    #[serde(rename = "_id")]
    pub id: Id,
    pub key: Id,
    pub name: String,
    pub r#type: String,
    pub group: String,
    pub topic: String,
    pub environment: Environment,
    pub body: String,
    #[serde(with = "http_serde_ext_ios::header_map")]
    pub headers: HeaderMap,
    #[serde(with = "chrono::serde::ts_milliseconds")]
    pub arrived_at: DateTime<Utc>,
    pub arrived_date: DateTime<Utc>,
    pub state: EventState,
    pub ownership: Ownership,
    pub hashes: [HashValue; 3],
    pub payload_byte_length: usize,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub duplicates: Option<Duplicates>,
    #[serde(flatten, default)]
    pub record_metadata: RecordMetadata,
}

struct IntermediateEventFields<'a> {
    access_key: &'a AccessKey,
    encrypted_access_key: &'a EncryptedAccessKey<'a>,
    event_name: &'a str,
    headers: HeaderMap,
    body: String,
    timestamp: DateTime<Utc>,
    id: Id,
    key: Id,
}

impl Event {
    pub fn new(
        access_key: &AccessKey,
        encrypted_access_key: &EncryptedAccessKey,
        event_name: &str,
        headers: HeaderMap,
        body: String,
    ) -> Self {
        let timestamp = Utc::now().round_subsecs(3);
        let id = Id::new(IdPrefix::Event, timestamp);
        let key = Id::new(IdPrefix::EventKey, timestamp);
        let fields = IntermediateEventFields {
            access_key,
            encrypted_access_key,
            event_name,
            headers,
            body,
            timestamp,
            id,
            key,
        };
        Self::new_with_timestamp_and_ids(fields)
    }

    pub fn add_duplicates(mut self, duplicates: Duplicates) -> Self {
        self.duplicates = Some(duplicates);
        self
    }

    fn new_with_timestamp_and_ids(fields: IntermediateEventFields<'_>) -> Self {
        let topic = fields.access_key.get_topic(fields.event_name);
        let access_key_data = &fields.access_key.data;
        let environment = fields.access_key.prefix.environment;
        let state = EventState::Pending;
        let buildable_id = access_key_data.id.to_owned();
        let ownership = Ownership {
            id: buildable_id.clone().into(),
            client_id: buildable_id.to_owned(),
            organization_id: Some(buildable_id.to_owned()),
            project_id: Some(buildable_id.to_owned()),
            user_id: Some(buildable_id),
        };
        let hashes = Hashes::new(
            &topic,
            environment,
            &fields.body,
            &access_key_data.event_type,
            &access_key_data.group,
        );
        let hashes = hashes.get_hashes();

        let payload_byte_length = fields.body.len();
        Event {
            id: fields.id,
            key: fields.key,
            name: fields.event_name.to_owned(),
            r#type: fields.access_key.data.event_type.clone(),
            group: fields.access_key.data.group.clone(),
            access_key: fields.encrypted_access_key.to_string(),
            topic,
            environment,
            body: fields.body,
            headers: fields.headers,
            arrived_at: fields.timestamp,
            arrived_date: fields.timestamp,
            state,
            ownership,
            hashes,
            payload_byte_length,
            duplicates: None,
            record_metadata: Default::default(),
        }
    }

    pub fn to_public(self) -> PublicEvent {
        PublicEvent {
            id: self.id,
            key: self.key,
            name: self.name.clone(),
            r#type: self.r#type.clone(),
            group: self.group.clone(),
            topic: self.topic.clone(),
            environment: self.environment,
            body: self.body.clone(),
            headers: self.headers.clone(),
            arrived_at: self.arrived_at,
            arrived_date: self.arrived_date,
            state: self.state,
            ownership: self.ownership.clone(),
            hashes: self.hashes,
            payload_byte_length: self.payload_byte_length,
            duplicates: self.duplicates.clone(),
            record_metadata: self.record_metadata.clone(),
        }
    }
}

#[cfg(test)]
mod test {

    use crate::prelude::access_key::{
        access_key_data::AccessKeyData, access_key_prefix::AccessKeyPrefix, event_type::EventType,
    };

    use super::*;
    use chrono::TimeZone;
    use http::{HeaderMap, HeaderValue};
    use once_cell::sync::Lazy;
    use test::hashes::HashType;
    use uuid::Uuid;

    static HEADERS: Lazy<HeaderMap> = Lazy::new(|| {
        let mut headers = HeaderMap::new();
        headers.insert("foo", HeaderValue::from_static("bar"));
        headers.insert("baz", HeaderValue::from_static("qux"));
        headers
    });

    static ACCESS_KEY: Lazy<AccessKey> = Lazy::new(|| AccessKey {
        prefix: AccessKeyPrefix {
            environment: Environment::Test,
            event_type: EventType::Id,
            version: 1,
        },
        data: AccessKeyData {
            id: "foo".to_owned(),
            event_type: "bar".to_owned(),
            group: "baz".to_owned(),
            namespace: "qux".to_owned(),
            event_path: "quux".to_owned(),
            event_object_id_path: None,
            timestamp_path: None,
            parent_access_key: None,
        },
    });

    #[test]
    fn test_event_with_timestamp_and_ids() {
        let timestamp = Utc.timestamp_opt(0, 0).single().unwrap();
        let event_id = Id::new_with_uuid(IdPrefix::Event, timestamp, Uuid::nil());
        let key_id = Id::new_with_uuid(IdPrefix::EventKey, timestamp, Uuid::nil());
        let event_name = "event.received";
        let event = Event::new_with_timestamp_and_ids(IntermediateEventFields {
            access_key: &ACCESS_KEY,
            encrypted_access_key: &EncryptedAccessKey::parse("id_live_1_foo").unwrap(),
            event_name,
            headers: HEADERS.clone(),
            body: "hello world".to_owned(),
            timestamp,
            id: event_id,
            key: key_id,
        });
        assert_eq!(event.topic, ACCESS_KEY.get_topic(event_name));
        assert_eq!(event.environment, Environment::Test);
        assert_eq!(event.body, "hello world");
        assert_eq!(event.headers, *HEADERS);
        assert_eq!(event.arrived_at, timestamp);
        assert_eq!(event.state, EventState::Pending);
        assert_eq!(
            event.ownership,
            Ownership {
                id: ACCESS_KEY.data.id.to_owned().into(),
                client_id: ACCESS_KEY.data.id.to_owned(),
                organization_id: Some(ACCESS_KEY.data.id.to_owned()),
                project_id: Some(ACCESS_KEY.data.id.to_owned()),
                user_id: Some(ACCESS_KEY.data.id.to_owned())
            }
        );
        assert_eq!(
            event.hashes,
            [
                HashValue {
                    r#type: HashType::Body,
                    hash: "39c898e492b3eadc9798e23e28d8f89392c584ef4e495992e08a146d6b71a535"
                        .to_owned(),
                },
                HashValue {
                    r#type: HashType::Event,
                    hash: "fb6d7839ce31c8a72e3f9396c569bff26af7e10e361d9a731b813ec9a60693be"
                        .to_owned(),
                },
                HashValue {
                    r#type: HashType::ModelBody,
                    hash: "85ac81f9ee4268c027c6b35f4dbc613673280630ee85e676c005a5fe69b3be63"
                        .to_owned(),
                },
            ]
        );
        assert_eq!(event.payload_byte_length, 11);
    }

    #[test]
    fn test_event_serde() {
        let event = Event::new(
            &ACCESS_KEY,
            &EncryptedAccessKey::parse("id_live_1_foo").unwrap(),
            "event.received",
            HEADERS.clone(),
            "hello world".to_owned(),
        );
        let json = serde_json::to_string(&event).unwrap();
        let deserialized: Event = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, event);
    }
}
