use anyhow::Result;
use integrationos_domain::event_access::EventAccess;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tower_governor::{errors::GovernorError, key_extractor::KeyExtractor};

#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq)]
pub struct OwnershipId;

impl KeyExtractor for OwnershipId {
    type Key = String;

    fn extract<T>(&self, req: &http::request::Request<T>) -> Result<Self::Key, GovernorError> {
        let event_access = req
            .extensions()
            .get::<Arc<EventAccess>>()
            .ok_or_else(|| GovernorError::UnableToExtractKey)?;

        Ok(event_access.ownership.id.to_string())
    }
}
