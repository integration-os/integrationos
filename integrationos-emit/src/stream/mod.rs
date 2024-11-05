pub mod fluvio_driver;

use crate::domain::event::EventEntity;
use async_trait::async_trait;
use integrationos_domain::{Id, IntegrationOSError, Unit};
use strum::{AsRefStr, EnumString};
use tokio_util::sync::CancellationToken;

#[async_trait]
pub trait EventStreamExt {
    async fn publish(&self, event: EventEntity) -> Result<Id, IntegrationOSError>;
    async fn consume(&self, token: CancellationToken) -> Result<Unit, IntegrationOSError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumString, AsRefStr)]
#[strum(serialize_all = "kebab-case")]
pub enum EventStreamProvider {
    Fluvio,
    Logger,
}
