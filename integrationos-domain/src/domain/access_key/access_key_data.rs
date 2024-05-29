use crate::{IntegrationOSError, InternalError};
use napi_derive::napi;
use prost::Message;

#[derive(Clone, Eq, PartialEq, prost::Message)]
#[napi(object)]
#[cfg_attr(feature = "dummy", derive(fake::Dummy))]
pub struct AccessKeyData {
    #[prost(string, tag = "1")]
    pub id: String,
    #[prost(string, tag = "2")]
    pub namespace: String,
    #[prost(string, tag = "3")]
    pub event_type: String,
    #[prost(string, tag = "4")]
    pub group: String,
    #[prost(string, tag = "5")]
    pub event_path: String,
    #[prost(string, optional, tag = "6")]
    pub event_object_id_path: Option<String>,
    #[prost(string, optional, tag = "7")]
    pub timestamp_path: Option<String>,
    #[prost(string, optional, tag = "8")]
    pub parent_access_key: Option<String>,
}

impl TryFrom<&[u8]> for AccessKeyData {
    type Error = IntegrationOSError;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        Self::from_slice(value)
    }
}

impl AccessKeyData {
    pub fn from_slice(vec: &[u8]) -> Result<Self, IntegrationOSError> {
        AccessKeyData::decode(vec).map_err(|e| {
            InternalError::decryption_error(
                &e.to_string(),
                Some("Failed to decode access key data"),
            )
        })
    }

    pub fn to_vec(&self) -> Result<Vec<u8>, IntegrationOSError> {
        let mut buf = Vec::with_capacity(self.encoded_len());
        self.encode(&mut buf).map_err(|e| {
            InternalError::encryption_error(
                &e.to_string(),
                Some("Failed to encode access key data"),
            )
        })?;
        Ok(buf)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_access_key_data() {
        let access_key_data = AccessKeyData {
            id: "foo".to_owned(),
            namespace: "bar".to_owned(),
            event_type: "baz".to_owned(),
            group: "qux".to_owned(),
            event_path: "quux".to_owned(),
            event_object_id_path: Some("quuz".to_owned()),
            timestamp_path: None,
            parent_access_key: None,
        };
        let vec = access_key_data.to_vec().unwrap();
        assert_eq!(access_key_data, AccessKeyData::from_slice(&vec).unwrap());
    }
}
