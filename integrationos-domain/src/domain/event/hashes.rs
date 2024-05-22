use serde::{Deserialize, Serialize};
use sha3::{Digest, Keccak256};

use crate::prelude::configuration::environment::Environment;

const HASH_PREFIX: &str = "\x19Buildable Signed Message:\n";

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
pub struct Hashes<'a> {
    topic: &'a str,
    environment: Environment,
    body: &'a str,
    #[serde(skip)]
    r#type: &'a str,
    #[serde(skip)]
    group: &'a str,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize, Hash)]
#[cfg_attr(feature = "dummy", derive(fake::Dummy))]
#[serde(rename_all = "camelCase")]
pub enum HashType {
    Body,
    Event,
    #[serde(rename = "model::body")]
    ModelBody,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize, Hash)]
#[cfg_attr(feature = "dummy", derive(fake::Dummy))]
pub struct HashValue {
    #[serde(rename = "type")]
    pub r#type: HashType,
    pub hash: String,
}

impl<'a> Hashes<'a> {
    pub fn new(
        topic: &'a str,
        environment: Environment,
        body: &'a str,
        r#type: &'a str,
        group: &'a str,
    ) -> Self {
        Self {
            topic,
            environment,
            body,
            r#type,
            group,
        }
    }

    pub fn get_hashes(&self) -> [HashValue; 3] {
        [
            HashValue {
                r#type: HashType::Body,
                hash: self.get_body_hash(),
            },
            HashValue {
                r#type: HashType::Event,
                hash: self.get_event_hash(),
            },
            HashValue {
                r#type: HashType::ModelBody,
                hash: self.get_model_body_hash(),
            },
        ]
    }

    fn get_event_hash(&self) -> String {
        // This serde_json::to_string is safe because Hashes implements Serialize
        // so in practice this will never panic
        Self::get_hash(&serde_json::to_string(self).unwrap())
    }

    fn get_body_hash(&self) -> String {
        Self::get_hash(self.body)
    }

    fn get_model_body_hash(&self) -> String {
        Self::get_hash(format!("{}:{}:{}", self.r#type, self.group, self.body).as_str())
    }

    fn get_hash(message: &str) -> String {
        let mut hasher = Keccak256::new();
        hasher.update(HASH_PREFIX);
        hasher.update(message.len().to_string());
        hasher.update(message);
        format!("{:x}", hasher.finalize())
    }
}

#[cfg(test)]
mod test {
    use once_cell::sync::Lazy;

    use super::*;

    const HASHES: Hashes<'static> = Hashes {
        topic: "foo",
        environment: Environment::Test,
        body: "bar",
        r#type: "baz",
        group: "qux",
    };

    static HASH_VALUES: Lazy<[HashValue; 3]> = Lazy::new(|| {
        [
            HashValue {
                r#type: HashType::Body,
                hash: "10e00c74fa981f00a807505ade917fe8dd54452a585422fd8e90842661712ec5".to_owned(),
            },
            HashValue {
                r#type: HashType::Event,
                hash: "8d0a4cf1b48d755c25cf1a5846c0ed5ae156e0dea04f365e4472d6b09aad1f8d".to_owned(),
            },
            HashValue {
                r#type: HashType::ModelBody,
                hash: "91c8ac33009f10bdf8ca6f29f47b77e98decb31ecb11c027c8d67689a94bc1e6".to_owned(),
            },
        ]
    });

    #[test]
    fn test_hashes_get_hash_types() {
        let body_hash = HASHES.get_body_hash();
        let event_hash = HASHES.get_event_hash();
        let model_body_hash = HASHES.get_model_body_hash();
        let hashes = HASHES.get_hashes();
        assert_eq!(
            hashes[0],
            HashValue {
                r#type: HashType::Body,
                hash: body_hash
            }
        );
        assert_eq!(
            hashes[1],
            HashValue {
                r#type: HashType::Event,
                hash: event_hash
            }
        );
        assert_eq!(
            hashes[2],
            HashValue {
                r#type: HashType::ModelBody,
                hash: model_body_hash
            }
        );
    }

    #[test]
    fn test_hashes_get_hashes() {
        let hashes = HASHES.get_hashes();
        assert_eq!(hashes, *HASH_VALUES);
    }

    #[test]
    fn test_hashes_serialize() {
        let hashes = HASHES.get_hashes();
        let serialized = serde_json::to_string(&hashes).unwrap();
        assert_eq!(serialized, r#"[{"type":"body","hash":"10e00c74fa981f00a807505ade917fe8dd54452a585422fd8e90842661712ec5"},{"type":"event","hash":"8d0a4cf1b48d755c25cf1a5846c0ed5ae156e0dea04f365e4472d6b09aad1f8d"},{"type":"model::body","hash":"91c8ac33009f10bdf8ca6f29f47b77e98decb31ecb11c027c8d67689a94bc1e6"}]"#.to_owned());
        let deserialized: [HashValue; 3] = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized, *HASH_VALUES);
    }
}
