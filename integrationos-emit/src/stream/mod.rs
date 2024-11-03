pub mod fluvio_driver;

use crate::domain::event::EventEntity;
use async_trait::async_trait;
use integrationos_domain::{Id, IntegrationOSError};
use strum::{AsRefStr, EnumString};

#[async_trait]
pub trait EventStreamExt {
    async fn publish(&self, event: EventEntity) -> Result<Id, IntegrationOSError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumString, AsRefStr)]
#[strum(serialize_all = "kebab-case")]
pub enum EventStreamProvider {
    Fluvio,
    Logger,
}
