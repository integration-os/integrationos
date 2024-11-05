pub mod fluvio_driver;

use crate::{domain::event::EventEntity, server::AppState};
use async_trait::async_trait;
use fluvio::consumer::Record;
use integrationos_domain::{Id, IntegrationOSError, Unit};
use strum::{AsRefStr, EnumString};
use tokio_util::sync::CancellationToken;

#[async_trait]
pub trait EventStreamExt<T = Record> {
    async fn publish(&self, event: EventEntity, is_dlq: bool) -> Result<Id, IntegrationOSError>;
    async fn consume(
        &self,
        token: CancellationToken,
        ctx: &AppState,
    ) -> Result<Unit, IntegrationOSError>;
    async fn process(&self, ctx: &AppState, event: &T) -> Result<Unit, IntegrationOSError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumString, AsRefStr)]
#[strum(serialize_all = "kebab-case")]
pub enum EventStreamProvider {
    Fluvio,
    Logger,
}
