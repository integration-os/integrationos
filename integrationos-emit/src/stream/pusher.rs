use super::EventStreamExt;
use crate::{
    domain::{config::EmitterConfig, deduplication::Deduplication, event::EventEntity},
    stream::EventStreamTopic,
};
use chrono::{Duration as CDuration, Utc};
use futures::{StreamExt, TryStreamExt};
use integrationos_domain::{IntegrationOSError, InternalError, MongoStore, Unit};
use mongodb::bson::doc;
use std::{sync::Arc, time::Duration};

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
    pub async fn start(&self, config: &EmitterConfig) -> Result<Unit, IntegrationOSError> {
        let events_store = self.events.clone();
        let deduplication_store = self.deduplication.clone();
        let event_stream = Arc::clone(&self.event_stream);

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
                    ]},
                    {"$and": [
                        {
                            "outcome.type": "executed"
                        },
                        {
                            "createdAt": { "$lt": before.timestamp_millis() }
                        }
                    ]},
                    {"$and": [
                        {
                            "outcome.type": "created"
                        },
                        {
                            "createdAt": { "$lt": before.timestamp_millis() }
                        }
                    ]}
                ]
            };

            tracing::info!("Querying for events: {query}");

            let events = events_store.collection.find(query, None).await;

            if let Ok(events) = events {
                let event_stream = Arc::clone(&event_stream);
                let deduplication_store = deduplication_store.clone();
                let results =
                    events
                        .try_chunks(max_chunk_size)
                        .map(|result| {
                            let event_stream = Arc::clone(&event_stream);
                            let deduplication_store = deduplication_store.clone();

                            let result =
                                result.map_err(|e| InternalError::io_err(&e.to_string(), None));
                            async move {
                                process_chunk(result, &event_stream, &deduplication_store).await
                            }
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
    result: Result<Vec<EventEntity>, IntegrationOSError>,
    event_stream: &Arc<dyn EventStreamExt + Sync + Send>,
    deduplication_store: &MongoStore<Deduplication>,
) -> Result<Unit, IntegrationOSError> {
    match result {
        Ok(chunk) => {
            tracing::info!("Publishing {} event(s)", chunk.len());
            for event in chunk {
                let entity_id = event.entity_id;
                let topic = if event.is_created() {
                    EventStreamTopic::Target
                } else {
                    EventStreamTopic::Dlq
                };

                let deleted = deduplication_store
                    .collection
                    .delete_one(doc! { "_id": entity_id.to_string() }, None)
                    .await?;

                tracing::info!(
                    "Deleted event with id {:?} from deduplication store",
                    deleted
                );

                event_stream
                    .publish(event, topic)
                    .await
                    .inspect_err(|e| {
                        tracing::error!("Failed to publish event: {e}");
                    })
                    .inspect(|_| {
                        tracing::info!("Event with id {} is published", entity_id);
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
