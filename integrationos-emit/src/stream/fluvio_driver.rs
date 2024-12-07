use super::{EventStreamExt, EventStreamTopic};
use crate::{
    domain::{
        config::{EmitterConfig, EventStreamConfig},
        deduplication::Deduplication,
        event::{EventEntity, EventStatus},
    },
    server::AppState,
};
use anyhow::Context;
use async_trait::async_trait;
use fluvio::{
    consumer::{
        ConsumerConfigExt, ConsumerConfigExtBuilder, ConsumerStream, OffsetManagementStrategy,
        Record,
    },
    dataplane::link::ErrorCode,
    spu::SpuSocketPool,
    Compression, Fluvio, FluvioConfig, Offset, RetryPolicy, TopicProducer,
    TopicProducerConfigBuilder,
};
use futures::StreamExt;
use integrationos_domain::{Id, IntegrationOSError, InternalError, TimedExt, Unit};
use mongodb::bson::doc;
use std::boxed::Box;
use std::{
    sync::atomic::{AtomicBool, AtomicU64, Ordering},
    time::Duration,
};
use tokio::time::interval;
use tokio_graceful_shutdown::SubsystemHandle;

pub struct ConsumerConfig {
    ext: ConsumerConfigExt,
    app: EventStreamConfig,
}

type TargetProducer = TopicProducer<SpuSocketPool>;
type DlqProducer = TopicProducer<SpuSocketPool>;

pub struct FluvioDriverImpl {
    pub client: Fluvio,
    pub tgt_consumer: ConsumerConfig,
    pub dlq_consumer: ConsumerConfig,
    pub tgt_producer: TargetProducer,
    pub dlq_producer: DlqProducer,
}

impl FluvioDriverImpl {
    pub async fn new(config: &EmitterConfig) -> Result<Self, IntegrationOSError> {
        let fluvio_config = FluvioConfig::new(config.fluvio.endpoint());
        let fluvio_client = Fluvio::connect_with_config(&fluvio_config).await?;

        let tgt_producer = match &config.fluvio.producer_topic {
            Some(producer_topic) => {
                // TODO: Bring the retry policy from the config
                let config = TopicProducerConfigBuilder::default()
                    .batch_size(config.fluvio.producer_batch_size)
                    .linger(Duration::from_millis(config.fluvio.producer_linger_time))
                    .delivery_semantic(
                        fluvio::DeliverySemantic::AtLeastOnce(RetryPolicy::default()),
                    )
                    .compression(Compression::Gzip)
                    .build()
                    .map_err(|e| anyhow::anyhow!("Could not create producer config: {e}"))?;

                fluvio_client
                    .topic_producer_with_config(producer_topic, config)
                    .await?
            }
            None => {
                return Err(InternalError::configuration_error(
                    "Producer not initialized",
                    None,
                ))
            }
        };

        let dlq_producer = {
            let topic = config.fluvio.dlq_topic.clone();
            let config = TopicProducerConfigBuilder::default()
                .batch_size(config.fluvio.producer_batch_size)
                .linger(Duration::from_millis(config.fluvio.producer_linger_time))
                .delivery_semantic(fluvio::DeliverySemantic::AtLeastOnce(RetryPolicy::default()))
                .compression(Compression::Gzip)
                .build()
                .map_err(|e| anyhow::anyhow!("Could not create producer config: {e}"))?;

            fluvio_client
                .topic_producer_with_config(&topic, config)
                .await?
        };

        let tgt_consumer = match &config.fluvio.consumer_topic {
            Some(consumer_topic) => {
                let offset = match &config.fluvio.absolute_offset {
                    Some(absolute_offset) => Offset::absolute(*absolute_offset).map_err(|e| {
                        InternalError::invalid_argument(
                            &format!("Could not create offset: {e}"),
                            None,
                        )
                    })?,
                    None => Offset::beginning(),
                };

                let consumer_id = config.fluvio.consumer_group.clone().ok_or_else(|| {
                    InternalError::invalid_argument(
                        "When specifying a consumer topic, a consumer group must be specified",
                        None,
                    )
                })?;

                let mut ext = ConsumerConfigExtBuilder::default();

                if let Some(partition) = config.partition() {
                    ext = ext.partition(partition).to_owned();
                }

                let ext = ext
                    .topic(consumer_topic)
                    .offset_start(offset)
                    .offset_consumer(consumer_id)
                    .offset_strategy(OffsetManagementStrategy::Manual)
                    .build()
                    .map_err(|e| anyhow::anyhow!("Could not create consumer config: {e}"))?;

                ConsumerConfig {
                    ext,
                    app: config.fluvio.clone(),
                }
            }
            None => {
                return Err(InternalError::configuration_error(
                    "Consumer not initialized",
                    None,
                ))
            }
        };

        let dlq_consumer = {
            let topic = config.fluvio.dlq_topic.clone();
            let consumer_id = config.fluvio.consumer_group.clone().ok_or_else(|| {
                InternalError::invalid_argument(
                    "When specifying a consumer topic, a consumer group must be specified",
                    None,
                )
            })?;

            let consumer_id = format!("{consumer_id}-dlq");

            let mut ext = ConsumerConfigExtBuilder::default();

            if let Some(partition) = config.partition() {
                ext = ext.partition(partition).to_owned();
            }

            let ext = ext
                .topic(&topic)
                .offset_start(Offset::beginning())
                .offset_consumer(consumer_id)
                .offset_strategy(OffsetManagementStrategy::Manual)
                .build()
                .map_err(|e| anyhow::anyhow!("Could not create consumer config: {e}"))?;

            ConsumerConfig {
                ext,
                app: config.fluvio.clone(),
            }
        };

        Ok(Self {
            client: fluvio_client,
            tgt_consumer,
            dlq_consumer,
            tgt_producer,
            dlq_producer,
        })
    }

    async fn consume_topic(
        &self,
        target: EventStreamTopic,
        subsys: &SubsystemHandle,
        ctx: &AppState,
        consumer: &ConsumerConfig,
        stream: &mut impl ConsumerStream<Item = Result<Record, ErrorCode>>,
    ) -> Result<Unit, IntegrationOSError> {
        let mut interval = interval(Duration::from_millis(consumer.app.consumer_linger_time));
        interval.tick().await;

        // We don't really need it but we may use a different approach if something comes out of https://github.com/infinyon/fluvio/issues/4267#issuecomment-2489354987
        let count = AtomicU64::new(0);
        let is_processing = AtomicBool::new(true);

        if !consumer.ext.partition.is_empty() {
            tracing::info!(
                "Consuming events from topic {} partition {}",
                target.as_ref(),
                consumer
                    .ext
                    .partition
                    .iter()
                    .map(u32::to_string)
                    .collect::<Vec<_>>()
                    .join("-")
            );
        }

        loop {
            is_processing.store(false, Ordering::SeqCst);
            tokio::select! {
                timeout = interval.tick() => {
                    if count.load(std::sync::atomic::Ordering::SeqCst) > 0 {
                        tracing::info!("Committing offsets after {:?} for topic {}", timeout.elapsed(), target.as_ref());
                        stream.offset_commit().map_err(|err| anyhow::anyhow!(err))?;
                        stream.offset_flush().await.map_err(|err| anyhow::anyhow!(err))?;
                        tracing::info!("Periodic offset commit completed for topic {}", target.as_ref());
                        count.store(0, std::sync::atomic::Ordering::SeqCst);
                    }

                    if subsys.is_shutdown_requested() && !is_processing.load(Ordering::SeqCst) {
                        tracing::info!("Consumer for {} cancelled by external request. Breaking the loop", target.as_ref());
                        break Ok(());
                    }
                },
                record = stream.next() => {
                    count.fetch_add(1, Ordering::Relaxed);

                    match record {
                        Some(Ok(record)) => {
                            let event: EventEntity = serde_json::from_slice(record.get_value()).context("Could not deserialize event")?;
                            is_processing.store(true, Ordering::SeqCst);
                            self.process(ctx, target, &event).await?;
                            is_processing.store(false, Ordering::SeqCst);
                        },
                        Some(Err(err)) => return Err(InternalError::io_err(&format!("Error consuming record: {err}"), None)),
                        None => {
                            tracing::info!("Consumer stream closed");
                            subsys.request_shutdown();
                        }
                    }

                    if count.load(std::sync::atomic::Ordering::SeqCst) >= consumer.app.consumer_batch_size as u64 {
                        count.store(0, Ordering::SeqCst);
                        stream.offset_commit().map_err(|err| anyhow::anyhow!(err))?;
                        stream.offset_flush().await.map_err(|err| anyhow::anyhow!(err))?;
                    }

                    if subsys.is_shutdown_requested() {
                        tracing::info!("Consumer for {} cancelled by external request. Breaking the loop", target.as_ref());
                        break Ok(());
                    }
                }
            }
        }
    }
}

#[async_trait]
impl EventStreamExt for FluvioDriverImpl {
    /**
     * Publishes an event to the specified topic
     * @param event - The event to publish
     * @param target - The target topic of the event
     *
     * It serializes the event using serde_json and sends it to the specified topic.
     */
    async fn publish(
        &self,
        event: EventEntity,
        target: EventStreamTopic,
    ) -> Result<Id, IntegrationOSError> {
        let payload = serde_json::to_vec(&event).map_err(|e| {
            InternalError::serialize_error(&format!("Could not serialize event: {e}"), None)
        })?;

        match target {
            EventStreamTopic::Target => {
                self.tgt_producer
                    .send(event.entity_id.to_string(), payload)
                    .await
                    .map_err(|e| {
                        InternalError::io_err(&format!("Could not send event to fluvio: {e}"), None)
                    })?;
            }
            EventStreamTopic::Dlq => {
                self.dlq_producer
                    .send(event.entity_id.to_string(), payload)
                    .await
                    .map_err(|e| {
                        InternalError::io_err(&format!("Could not send event to fluvio: {e}"), None)
                    })?;
            }
        };

        Ok(event.entity_id)
    }

    /**
     * Consumes events from the specified topic
     * @param target - The target topic of the event
     * @param subsys - The subsystem handle
     * @param ctx - The application state
     *
     * It consumes events from the specified topic using the consumer stream.
     * It processes each event and updates the event outcome in the events collection.
     */
    async fn consume(
        &self,
        target: EventStreamTopic,
        subsys: SubsystemHandle,
        ctx: &AppState,
    ) -> Result<Unit, IntegrationOSError> {
        let consumer = match target {
            EventStreamTopic::Target => &self.tgt_consumer,
            EventStreamTopic::Dlq => &self.dlq_consumer,
        };

        let mut stream = self
            .client
            .consumer_with_config(consumer.ext.clone())
            .await?;

        // match
        self.consume_topic(target, &subsys, ctx, consumer, &mut stream)
            .await
    }

    /**
     * Processes an event from the consumer stream
     * @param ctx - The application state
     * @param target - The target topic of the event
     * @param event - The event to process
     *
     * It first checks if the event is already processed, if so, it returns without processing it.
     * If the event is not processed, it executes the side effect and updates the event outcome.
     *
     * Finally, it updates the event outcome in the events collection if the side effect was executed at least once.
     */
    async fn process(
        &self,
        ctx: &AppState,
        target: EventStreamTopic,
        event: &EventEntity,
    ) -> Result<Unit, IntegrationOSError> {
        let is_processed = ctx
            .app_stores
            .deduplication
            .get_one_by_id(&event.entity_id.to_string())
            .await
            .map_err(|e| {
                tracing::error!("Could not fetch deduplication record: {e}");
                InternalError::unknown("Could not fetch deduplication record", None)
            })?
            .is_some();

        if is_processed {
            tracing::info!("Event with id {} is already processed", event.entity_id);
            return Ok(());
        }

        let insert_result = ctx
            .app_stores
            .deduplication
            .create_one(&Deduplication {
                entity_id: event.entity_id,
                metadata: event.metadata.clone(),
            })
            .await;

        if let Err(e) = insert_result {
            tracing::error!("Could not create deduplication record: {e}");
            if e.is_unique_error() {
                return Ok(());
            } else {
                return Err(e);
            }
        }

        match target {
            EventStreamTopic::Target => {
                ctx.app_stores.events.create_one(event).await.map_err(|e| {
                    tracing::error!("Could not create event record: {e}");
                    InternalError::unknown("Could not create event record", None)
                })?;

                tracing::info!("Event with id {} is ready to be processed", event.entity_id);
                let result = event
                    .side_effect(ctx)
                    .timed(|_, elapsed| {
                        tracing::info!(
                            "Side effect for entity id {} took {}ms",
                            event.entity_id,
                            elapsed.as_millis()
                        )
                    })
                    .await;

                update_event_outcome(ctx, event, EventStatus::executed()).await?;

                if let Err(e) = result {
                    tracing::error!("Error processing event: {e}, removing deduplication record");
                    delete_deduplication_record(ctx, event).await?;

                    let outcome = EventStatus::errored(e.to_string(), 1);
                    let event = event.with_outcome(outcome.clone());

                    self.publish(event.clone(), EventStreamTopic::Dlq).await?;

                    update_event_outcome(ctx, &event, outcome).await?;

                    return Ok(());
                }

                update_event_outcome(ctx, event, EventStatus::succeded(event.retries())).await?;
            }
            EventStreamTopic::Dlq => {
                tracing::info!("Event with id {} is in DLQ", event.entity_id);
                if event.retries() <= ctx.config.event_processing_max_retries {
                    let result = event.side_effect(ctx).await;

                    if let Err(e) = result {
                        tracing::error!(
                            "Error processing event: {e}, removing deduplication record"
                        );
                        delete_deduplication_record(ctx, event).await?;

                        let outcome = EventStatus::errored(e.to_string(), event.retries() + 1);
                        let event = event.with_outcome(outcome.clone());

                        self.publish(event.clone(), EventStreamTopic::Dlq).await?;

                        update_event_outcome(ctx, &event, outcome).await?;

                        return Ok(());
                    }

                    update_event_outcome(ctx, event, EventStatus::succeded(event.retries()))
                        .await?;
                } else {
                    tracing::info!("Giving up on event with id {}", event.entity_id);
                    // this is the case where we exhausted the retries, now
                    // the error is updated and not sent to the target topic
                    let error = event.error().unwrap_or_default()
                        + ".\n Exhausted retries, cannot process event";

                    update_event_outcome(ctx, event, EventStatus::errored(error, event.retries()))
                        .await?;

                    // TODO: create an alert on grafana
                }
            }
        }

        Ok(())
    }
}

async fn delete_deduplication_record(
    ctx: &AppState,
    event: &EventEntity,
) -> Result<Unit, IntegrationOSError> {
    ctx.app_stores
        .deduplication
        .collection
        .delete_one(doc! {
            "_id": event.entity_id.to_string()
        })
        .await?;

    Ok(())
}

async fn update_event_outcome(
    ctx: &AppState,
    event: &EventEntity,
    outcome: EventStatus,
) -> Result<Unit, IntegrationOSError> {
    let outcome = mongodb::bson::to_bson(&outcome).context("Could not serialize event")?;

    ctx.app_stores
        .events
        .update_one(
            &event.entity_id.to_string(),
            doc! { "$set": { "outcome": outcome } },
        )
        .await?;

    Ok(())
}
