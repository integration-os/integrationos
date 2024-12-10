use super::{EventStreamExt, SINGLETON_ID};
use crate::{
    domain::{config::EmitterConfig, deduplication::Deduplication, event::EventEntity},
    stream::EventStreamTopic,
};
use chrono::{Duration as CDuration, Utc};
use futures::{StreamExt, TryStreamExt};
use integrationos_domain::{IntegrationOSError, InternalError, MongoStore, Unit};
use mongodb::bson::doc;
use std::{sync::Arc, time::Duration};
use tokio_graceful_shutdown::{FutureExt, SubsystemHandle};

#[derive(Clone)]
pub struct EventPusher {
    pub event_stream: Arc<dyn EventStreamExt + Sync + Send>,
    pub events: MongoStore<EventEntity>,
    pub deduplication: MongoStore<Deduplication>,
    pub max_concurrent_tasks: usize,
    pub max_chunk_size: usize,
    pub sleep_duration: u64,
}

impl EventPusher {
    pub async fn start(
        &self,
        config: &EmitterConfig,
        subsys: SubsystemHandle,
    ) -> Result<Unit, IntegrationOSError> {
        if config.partition()? != SINGLETON_ID {
            tracing::info!(
                "Limiting event publisher to single partition {}",
                SINGLETON_ID
            );
            return Ok(());
        }

        match self.process(config).cancel_on_shutdown(&subsys).await {
            Ok(result) => {
                tracing::info!("Scheduled event publisher finished");
                subsys.on_shutdown_requested().await;

                result
            }
            Err(_) => {
                tracing::warn!("EventPusher was cancelled due to shutdown");
                subsys.on_shutdown_requested().await;
                Ok(())
            }
        }
    }

    async fn process(&self, config: &EmitterConfig) -> Result<Unit, IntegrationOSError> {
        let events_store = self.events.clone();
        let deduplication_store = self.deduplication.clone();
        let event_stream = Arc::clone(&self.event_stream);
        let claimer = config.partition()?;

        let max_concurrent_tasks = self.max_concurrent_tasks;
        let max_chunk_size = self.max_chunk_size;
        let sleep_duration = self.sleep_duration;

        tracing::info!("Starting event pusher");
        loop {
            let now = Utc::now();
            let before = now - CDuration::seconds(config.event_max_span_for_retry_secs);

            tracing::debug!("Polling for events at {}", now);

            let query = doc! {
                "$or": [
                    {"$and": [
                        {
                            "outcome.type": "errored"
                        },
                        {
                            "outcome.retries": { "$lt": config.event_processing_max_retries}
                        },
                        {
                            "createdAt": { "$lt": before.timestamp_millis() }
                        },
                        {
                            "claimedBy": { "$exists": false }
                        }
                    ]},
                    {"$and": [
                        {
                            "outcome.type": "executed"
                        },
                        {
                            "createdAt": { "$lt": before.timestamp_millis() }
                        },
                        {
                            "claimedBy": { "$exists": false }
                        }
                    ]},
                    {"$and": [
                        {
                            "outcome.type": "created"
                        },
                        {
                            "createdAt": { "$lt": before.timestamp_millis() }
                        },
                        {
                            "claimedBy": { "$exists": false }
                        }
                    ]}
                ]
            };

            let events = events_store.collection.find(query).await;

            if let Ok(events) = events {
                let event_stream = Arc::clone(&event_stream);
                let deduplication_store = deduplication_store.clone();
                let events_store = events_store.clone();

                let result = events
                    .try_chunks(max_chunk_size)
                    .map(|result| {
                        let event_stream = Arc::clone(&event_stream);
                        let deduplication_store = deduplication_store.clone();
                        let events_store = events_store.clone();

                        let result =
                            result.map_err(|e| InternalError::io_err(&e.to_string(), None));
                        async move {
                            process_chunk(
                                result,
                                &event_stream,
                                &deduplication_store,
                                &events_store,
                                claimer,
                            )
                            .await
                        }
                    })
                    .buffer_unordered(max_concurrent_tasks)
                    .collect::<Vec<_>>()
                    .await
                    .into_iter()
                    .collect::<Result<Vec<Unit>, IntegrationOSError>>();

                if let Err(e) = result {
                    tracing::error!("Failed to publish one or more event chunks: {e}");
                }
            } else if let Err(e) = events {
                tracing::error!("Failed to fetch events: {e}");
            }

            tokio::time::sleep(Duration::from_millis(sleep_duration)).await;
        }
    }
}

async fn process_chunk(
    result: Result<Vec<EventEntity>, IntegrationOSError>,
    event_stream: &Arc<dyn EventStreamExt + Sync + Send>,
    deduplication_store: &MongoStore<Deduplication>,
    events_store: &MongoStore<EventEntity>,
    claimer: u32,
) -> Result<Unit, IntegrationOSError> {
    match result {
        Ok(chunk) => {
            tracing::info!("Publishing {} event(s)", chunk.len());
            for event in chunk {
                // Double check mechanism to prevent duplicated events
                if events_store
                    .get_one_by_id(&event.entity_id.to_string())
                    .await?
                    .map(|e| e.claimed_by.is_some())
                    .unwrap_or(false)
                {
                    tracing::warn!("Event with id {} is already published", event.entity_id);
                    continue;
                }

                events_store
                    .update_one(
                        &event.entity_id.to_string(),
                        doc! { "$set": { "claimedBy": claimer } },
                    )
                    .await?;

                let entity_id = event.entity_id;

                let deleted = deduplication_store
                    .collection
                    .delete_one(doc! { "_id": entity_id.to_string() })
                    .await?;

                tracing::info!(
                    "Deleted event with id {:?} from deduplication store",
                    deleted
                );

                event_stream
                    .publish(event, EventStreamTopic::Dlq)
                    .await
                    .inspect(|_| {
                        tracing::info!("Event with id {} is published", entity_id);
                    })
                    .inspect_err(|e| {
                        tracing::error!("Failed to publish event: {e}");
                    })?;
            }
            Ok(())
        }
        Err(e) => {
            tracing::error!("Failed to chunk events: {e}");
            Err(e)
        }
    }
}
