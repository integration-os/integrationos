use crate::{hashed_secret::HashedSecret, IntegrationOSError, InternalError};
use serde_json::Value;
use sha3::{Digest, Keccak256};

pub trait HashExt {
    fn hash(&self, value: &str) -> Result<String, IntegrationOSError>;
    fn verify(&self, value: &str, hash: &str) -> bool;
}

pub struct HashKecAlgImpl;

impl HashKecAlgImpl {
    pub fn new() -> Self {
        HashKecAlgImpl
    }
}

impl Default for HashKecAlgImpl {
    fn default() -> Self {
        Self::new()
    }
}

impl HashExt for HashKecAlgImpl {
    fn hash(&self, value: &str) -> Result<String, IntegrationOSError> {
        let mut hasher = Keccak256::new();
        hasher.update(value);
        Ok(format!("{:x}", hasher.finalize()))
    }

    fn verify(&self, value: &str, hash: &str) -> bool {
        self.hash(value).ok().is_some_and(|h| h == hash)
    }
}

impl TryFrom<Value> for HashedSecret {
    type Error = IntegrationOSError;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        let value_str = serde_json::to_string(&value).map_err(|err| {
            InternalError::serialize_error(&format!("Failed to serialize value: {err}"), None)
        })?;
        let hash = HashKecAlgImpl::new().hash(&value_str)?;
        Ok(HashedSecret::new(hash))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_keccak256_hash() {
        let value = json!({
            "response": {
                "user": {
                    "name": "Alice",
                    "age": 3
                }
            }
        });

        let hash = HashKecAlgImpl::new().hash(&value.to_string()).unwrap();

        assert_eq!(
            hash,
            "eb42c0c05a0ac6cd15e4cf907a6aa913ebfe6aea79ee7edd054b519435f827cb"
        );
    }

    #[test]
    fn test_keccak256_verify() {
        let value = json!({
            "response": {
                "user": {
                    "name": "Alice",
                    "age": 3
                }
            }
        });

        let hash = HashKecAlgImpl::new().hash(&value.to_string()).unwrap();

        assert!(HashKecAlgImpl::new().verify(&value.to_string(), &hash));
    }

    #[test]
    fn test_hash_from_value() {
        let value = json!({
            "response": {
                "user": {
                    "name": "Alice",
                    "age": 3
                }
            }
        });

        let hash = HashedSecret::try_from(value).unwrap();

        assert_eq!(
            hash.inner(),
            "eb42c0c05a0ac6cd15e4cf907a6aa913ebfe6aea79ee7edd054b519435f827cb"
        );
    }
}
