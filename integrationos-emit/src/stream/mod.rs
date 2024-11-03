pub mod fluvio_driver;

use crate::domain::event::Event;
use async_trait::async_trait;
use integrationos_domain::{IntegrationOSError, Unit};
use strum::{AsRefStr, EnumString};

#[async_trait]
pub trait EventStreamExt {
    async fn publish(&self, event: Event) -> Result<Unit, IntegrationOSError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumString, AsRefStr)]
#[strum(serialize_all = "kebab-case")]
pub enum EventStreamProvider {
    Fluvio,
    Logger,
}
