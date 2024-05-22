use super::{access_key_prefix::AccessKeyPrefix, encrypted_data::EncryptedData};
use crate::{IntegrationOSError, InternalError};
use base64ct::{Base64UrlUnpadded, Encoding};
use std::{
    borrow::Cow,
    fmt::{Display, Formatter},
    hash::{Hash, Hasher},
    str,
};

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct EncryptedAccessKey<'a> {
    pub prefix: AccessKeyPrefix,
    data: Cow<'a, str>,
}

impl<'a> EncryptedAccessKey<'a> {
    pub fn new(prefix: AccessKeyPrefix, data: String) -> Self {
        Self {
            prefix,
            data: Cow::Owned(data),
        }
    }

    pub fn to_static(self) -> EncryptedAccessKey<'static> {
        match self.data {
            Cow::Borrowed(data) => EncryptedAccessKey {
                prefix: self.prefix,
                data: Cow::Owned(data.to_owned()),
            },
            Cow::Owned(data) => EncryptedAccessKey {
                prefix: self.prefix,
                data: Cow::Owned(data),
            },
        }
    }

    pub fn parse(access_key: &'a str) -> Result<Self, IntegrationOSError> {
        // Parse out the prefix, separated by '_'
        let mut parts = access_key.splitn(4, '_');
        let event_type = parts
            .next()
            .ok_or(InternalError::configuration_error(
                "No event type in access key",
                None,
            ))?
            .try_into()?;
        let environment = parts
            .next()
            .ok_or(InternalError::configuration_error(
                "No environment in access key",
                None,
            ))?
            .try_into()?;
        let version = parts
            .next()
            .ok_or(InternalError::configuration_error(
                "No version in access key",
                None,
            ))?
            .parse()
            .map_err(|e| {
                InternalError::configuration_error(&format!("Invalid version: {}", e), None)
            })?;

        let remainder = parts.next().ok_or(InternalError::configuration_error(
            "No remainder in access key",
            None,
        ))?;

        Ok(Self {
            prefix: AccessKeyPrefix::new(environment, event_type, version),
            data: Cow::Borrowed(remainder),
        })
    }

    pub fn get_encrypted_data(&self) -> Result<EncryptedData, IntegrationOSError> {
        // Take the rest of the key and decode it from base64url
        let remainder = Base64UrlUnpadded::decode_vec(&self.data).map_err(|e| {
            InternalError::configuration_error(&format!("Invalid base64url: {}", e), None)
        })?;
        if remainder.len() < 48 {
            return Err(InternalError::configuration_error(
                "Encrypted data (remainder) is too short",
                None,
            ));
        }
        Ok(EncryptedData::new(remainder))
    }
}

impl Hash for EncryptedAccessKey<'_> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.data.hash(state);
    }
}

impl Display for EncryptedAccessKey<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}_{}", self.prefix, self.data)
    }
}
