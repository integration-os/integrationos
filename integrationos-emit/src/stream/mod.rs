pub mod fluvio_driver;
pub mod logger_driver;
pub mod pusher;
pub mod scheduler;

use crate::{domain::event::EventEntity, server::AppState};
use async_trait::async_trait;
use integrationos_domain::{Id, IntegrationOSError, Unit};
use strum::{AsRefStr, Display, EnumIter, EnumString};
use tokio_graceful_shutdown::SubsystemHandle;

pub const SINGLETON_ID: u32 = 0;

#[async_trait]
pub trait EventStreamExt<T = EventEntity> {
    async fn publish(
        &self,
        event: EventEntity,
        target: EventStreamTopic,
    ) -> Result<Id, IntegrationOSError>;
    async fn consume(
        &self,
        target: EventStreamTopic,
        subsys: SubsystemHandle,
        ctx: &AppState,
    ) -> Result<Unit, IntegrationOSError>;
    async fn process(
        &self,
        ctx: &AppState,
        target: EventStreamTopic,
        events: &T,
    ) -> Result<Unit, IntegrationOSError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumString, AsRefStr, Display)]
#[strum(serialize_all = "kebab-case")]
pub enum EventStreamProvider {
    Fluvio,
    Logger,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumString, AsRefStr, EnumIter)]
#[strum(serialize_all = "kebab-case")]
pub enum EventStreamTopic {
    Target,
    Dlq,
}
