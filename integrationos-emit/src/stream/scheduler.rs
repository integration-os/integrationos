use super::{EventStreamExt, SINGLETON_ID};
use crate::{
    domain::{config::EmitterConfig, event::ScheduledEvent},
    stream::EventStreamTopic,
};
use chrono::Utc;
use futures::{StreamExt, TryStreamExt};
use integrationos_domain::{IntegrationOSError, InternalError, MongoStore, Unit};
use mongodb::bson::doc;
use std::{sync::Arc, time::Duration};
use tokio_graceful_shutdown::{FutureExt, SubsystemHandle};

// Simple scheduler. Heavily relies on the database for scheduling events
#[derive(Clone)]
pub struct PublishScheduler {
    pub event_stream: Arc<dyn EventStreamExt + Sync + Send>,
    pub scheduled: MongoStore<ScheduledEvent>,
    pub max_concurrent_tasks: usize,
    pub max_chunk_size: usize,
    pub sleep_duration: u64,
}

impl PublishScheduler {
    pub async fn start(
        &self,
        config: &EmitterConfig,
        subsys: SubsystemHandle,
    ) -> Result<Unit, IntegrationOSError> {
        if config.partition()? != SINGLETON_ID {
            tracing::info!(
                "Limiting event scheduler to single partition {}",
                SINGLETON_ID
            );
            return Ok(());
        }

        match self.process().cancel_on_shutdown(&subsys).await {
            Ok(result) => {
                tracing::info!("Scheduled event publisher finished");
                subsys.on_shutdown_requested().await;

                result
            }
            Err(_) => {
                tracing::warn!("PublishScheduler was cancelled due to shutdown");
                subsys.on_shutdown_requested().await;
                Ok(())
            }
        }
    }

    async fn process(&self) -> Result<Unit, IntegrationOSError> {
        let scheduled = self.scheduled.clone();
        let event_stream = Arc::clone(&self.event_stream);

        let max_concurrent_tasks = self.max_concurrent_tasks;
        let max_chunk_size = self.max_chunk_size;
        let sleep_duration = self.sleep_duration;

        tracing::info!("Starting scheduled event publisher");
        loop {
            tracing::debug!(
                "Polling for scheduled events at {}",
                Utc::now().timestamp_millis()
            );

            let events = scheduled
                .collection
                .find(doc! {
                            "scheduleOn": { "$lte": Utc::now().timestamp_millis() }

                })
                .await;

            if let Ok(events) = events {
                let event_stream = Arc::clone(&event_stream);
                let scheduled = scheduled.clone();
                let results = events
                    .try_chunks(max_chunk_size)
                    .map(|result| {
                        let event_stream = Arc::clone(&event_stream);
                        let scheduled = scheduled.clone();

                        let result =
                            result.map_err(|e| InternalError::io_err(&e.to_string(), None));
                        async move { process_chunk(result, &event_stream, &scheduled).await }
                    })
                    .buffer_unordered(max_concurrent_tasks)
                    .collect::<Vec<_>>()
                    .await;

                if results.iter().any(|r| r.is_err()) {
                    tracing::error!("Failed to publish one or more event chunks");
                }
            } else if let Err(e) = events {
                tracing::error!("Failed to fetch events: {e}");
            }

            tokio::time::sleep(Duration::from_millis(sleep_duration)).await;
        }
    }
}

async fn process_chunk(
    result: Result<Vec<ScheduledEvent>, IntegrationOSError>,
    event_stream: &Arc<dyn EventStreamExt + Sync + Send>,
    scheduled: &MongoStore<ScheduledEvent>,
) -> Result<Unit, IntegrationOSError> {
    match result {
        Ok(chunk) => {
            tracing::info!("Publishing {} scheduled event(s)", chunk.len());
            for event in chunk {
                let id = event.id;
                let entity_id = event.event.entity_id;
                if let Err(e) = event_stream
                    .publish(event.event, EventStreamTopic::Target)
                    .await
                {
                    tracing::error!("Failed to publish event: {e}");
                } else {
                    tracing::info!("Event with id {} is published", entity_id);
                    scheduled
                        .collection
                        .delete_one(doc! { "_id": id.to_string() })
                        .await?;
                }
            }
            Ok(())
        }
        Err(e) => {
            tracing::error!("Failed to chunk events: {e}");
            Err(e)
        }
    }
}
