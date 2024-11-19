use super::EventStreamExt;
use crate::{domain::event::ScheduledEvent, stream::EventStreamTopic};
use chrono::{Duration as CDuration, Utc};
use futures::{StreamExt, TryStreamExt};
use integrationos_domain::{IntegrationOSError, InternalError, MongoStore, Unit};
use mongodb::bson::doc;
use std::{sync::Arc, time::Duration};

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
    pub async fn start(&self, max_retries: u32) -> Result<Unit, IntegrationOSError> {
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

            let query = doc! {
                "$or": [
                    {
                        "scheduleOn": { "$lte": Utc::now().timestamp_millis() }
                    },
                    {"$and": [
                        {
                            "outcome.type": "errored"
                        },
                        {
                            "outcome.retries": { "$lt": max_retries }
                        }
                    ]},
                    {"$and": [
                        {
                            "outcome.type": "executed"
                        },
                        {
                            "createdAt": { "$lt": (Utc::now() - CDuration::days(1)).timestamp_millis() }
                        }
                    ]}
                ]
            };

            let events = scheduled.collection.find(query, None).await;

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
                        .delete_one(doc! { "_id": id.to_string() }, None)
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
