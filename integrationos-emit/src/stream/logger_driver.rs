use super::{EventStreamExt, EventStreamTopic};
use crate::{domain::event::EventEntity, server::AppState};
use async_trait::async_trait;
use fluvio::consumer::Record;
use integrationos_domain::{prefix::IdPrefix, Id, IntegrationOSError, Unit};
use std::boxed::Box;
use tokio_graceful_shutdown::SubsystemHandle;

pub struct LoggerDriverImpl;

#[async_trait]
impl EventStreamExt for LoggerDriverImpl {
    async fn publish(
        &self,
        event: EventEntity,
        _target: EventStreamTopic,
    ) -> Result<Id, IntegrationOSError> {
        tracing::info!("Received event: {:?}, using logger handler", event);

        Ok(Id::now(IdPrefix::PipelineEvent))
    }

    async fn consume(
        &self,
        target: EventStreamTopic,
        _subsys: SubsystemHandle,
        _ctx: &AppState,
    ) -> Result<Unit, IntegrationOSError> {
        tracing::info!(
            "Consuming records from {} using logger handler",
            target.as_ref()
        );

        Ok(())
    }

    async fn process(
        &self,
        _ctx: &AppState,
        target: EventStreamTopic,
        _event: &Record,
    ) -> Result<Unit, IntegrationOSError> {
        tracing::info!(
            "Processing records from {} using logger handler",
            target.as_ref()
        );

        Ok(())
    }
}
