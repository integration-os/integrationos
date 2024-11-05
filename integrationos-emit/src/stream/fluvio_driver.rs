use super::EventStreamExt;
use crate::{
    domain::{
        config::{EmitterConfig, StreamConfig},
        deduplication::Deduplication,
        event::{EventEntity, EventOutcome},
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
    spu::SpuSocketPool,
    Compression, Fluvio, FluvioConfig, Offset, RetryPolicy, TopicProducer,
    TopicProducerConfigBuilder,
};
use futures::StreamExt;
use integrationos_domain::{prefix::IdPrefix, Id, IntegrationOSError, InternalError, Unit};
use mongodb::bson::doc;
use std::boxed::Box;
use std::time::Duration;
use tokio_util::sync::CancellationToken;

pub struct ConsumerConfig {
    ext: ConsumerConfigExt,
    app: StreamConfig,
}

type EventProducer = TopicProducer<SpuSocketPool>;
type DlqProducer = TopicProducer<SpuSocketPool>;

pub struct FluvioDriverImpl {
    pub client: Fluvio,
    pub evt_consumer: ConsumerConfig,
    pub dql_consumer: ConsumerConfig,
    pub evt_producer: EventProducer,
    pub dlq_producer: DlqProducer,
}

impl FluvioDriverImpl {
    pub async fn new(config: &EmitterConfig) -> Result<Self, IntegrationOSError> {
        let fluvio_config = FluvioConfig::new(config.fluvio.endpoint());
        let fluvio_client = Fluvio::connect_with_config(&fluvio_config).await?;

        let evt_producer = match &config.fluvio.producer_topic {
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
                    // TODO: Use topic producer with config
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
                // TODO: Use topic producer with config
                .topic_producer_with_config(&topic, config)
                .await?
        };

        let evt_consumer = match &config.fluvio.consumer_topic {
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

                let ext = ConsumerConfigExtBuilder::default()
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

        let dql_consumer = {
            let topic = config.fluvio.dlq_topic.clone();
            let consumer_id = config.fluvio.consumer_group.clone().ok_or_else(|| {
                InternalError::invalid_argument(
                    "When specifying a consumer topic, a consumer group must be specified",
                    None,
                )
            })?;

            let consumer_id = format!("{consumer_id}-dlq");

            let ext = ConsumerConfigExtBuilder::default()
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
            evt_consumer,
            dql_consumer,
            evt_producer,
            dlq_producer,
        })
    }
}

#[async_trait]
impl EventStreamExt for FluvioDriverImpl {
    async fn publish(&self, event: EventEntity, is_dlq: bool) -> Result<Id, IntegrationOSError> {
        let payload = serde_json::to_vec(&event).map_err(|e| {
            InternalError::serialize_error(&format!("Could not serialize event: {e}"), None)
        })?;

        if is_dlq {
            self.dlq_producer
                .send(event.partition_key(), payload)
                .await
                .map_err(|e| {
                    InternalError::io_err(&format!("Could not send event to fluvio: {e}"), None)
                })?;
        } else {
            self.evt_producer
                .send(event.partition_key(), payload)
                .await
                .map_err(|e| {
                    InternalError::io_err(&format!("Could not send event to fluvio: {e}"), None)
                })?;
        };

        Ok(event.entity_id)
    }

    async fn consume(
        &self,
        token: CancellationToken,
        ctx: &AppState,
    ) -> Result<Unit, IntegrationOSError> {
        let mut stream = self
            .client
            .consumer_with_config(self.evt_consumer.ext.clone())
            .await?;
        let mut count = 0;
        let mut interval = tokio::time::interval(Duration::from_millis(
            self.evt_consumer.app.consumer_linger_time,
        ));
        interval.tick().await;

        loop {
            tokio::select! {
                timeout = interval.tick() => {
                    if count > 0 || token.is_cancelled() {
                        tracing::info!("Committing offsets after {timeout:?}");
                        stream.offset_commit().map_err(|err| anyhow::anyhow!(err))?;
                        stream.offset_flush().await.map_err(|err| anyhow::anyhow!(err))?;
                        tracing::info!("Periodic offset commit completed.");
                        count = 0; // Reset count after Committing

                    }

                    if token.is_cancelled() {
                        tracing::info!("Consumer cancelled, gracefully shutting down. Committing pending offsets");
                        return Ok(());
                    }

                },
                record = stream.next() => {
                    count += 1;

                    match record {
                        Some(Ok(record)) => {
                            self.process(ctx, &record).await?;
                        },
                        Some(Err(err)) => return Err(InternalError::io_err(&format!("Error consuming record: {err}"), None)),
                        None => tracing::info!("Consumer stream closed")
                    }

                    if count >= self.evt_consumer.app.consumer_batch_size || token.is_cancelled() {
                        count = 0;
                        stream.offset_commit().map_err(|err| anyhow::anyhow!(err))?;
                        stream.offset_flush().await.map_err(|err| anyhow::anyhow!(err))?;
                    }

                    if token.is_cancelled() {
                        tracing::info!("Consumer cancelled, gracefully shutting down. Committing pending offsets");
                        return Ok(());
                    }
                }
                _ = token.cancelled() => {
                        tracing::info!("Consumer cancelled, gracefully shutting down. Committing pending offsets");
                        stream.offset_commit().map_err(|err| anyhow::anyhow!(err))?;
                        stream.offset_flush().await.map_err(|err| anyhow::anyhow!(err))?;
                        return Ok(());
                    },
            }
        }
    }

    async fn process(&self, ctx: &AppState, event: &Record) -> Result<Unit, IntegrationOSError> {
        let event: EventEntity =
            serde_json::from_slice(event.get_value()).context("Could not deserialize event")?;

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

        ctx.app_stores
            .events
            .create_one(&event)
            .await
            .map_err(|e| {
                tracing::error!("Could not create event record: {e}");
                InternalError::unknown("Could not create event record", None)
            })?;

        ctx.app_stores
            .deduplication
            .create_one(&Deduplication {
                entity_id: event.entity_id,
                metadata: event.metadata.clone(),
            })
            .await
            .map_err(|e| {
                tracing::error!("Could not create deduplication record: {e}");
                InternalError::unknown("Could not create deduplication record", None)
            })?;

        let result = event.side_effect(ctx).await;

        if let Err(e) = result {
            tracing::error!("Error processing event: {e}, removing deduplication record");
            ctx.app_stores
                .deduplication
                .collection
                .delete_one(
                    doc! {
                        "_id": event.entity_id.to_string()
                    },
                    None,
                )
                .await?;

            self.publish(event.clone(), true).await.inspect_err(|e| {
                tracing::error!("Could not publish event to DLQ: {e}");
            })?;

            let event = event.outcome(EventOutcome {
                success: false,
                retries: 1,
                error: Some(e.to_string()),
            });

            let outcome =
                mongodb::bson::to_bson(&event.outcome).context("Could not serialize outcome")?;

            ctx.app_stores
                .events
                .update_one(
                    &event.entity_id.to_string(),
                    doc! { "$set": { "outcome": outcome } },
                )
                .await?;

            return Ok(());
        }

        Ok(())
    }
}

pub struct FluvioDriverLogger;

#[async_trait]
impl EventStreamExt for FluvioDriverLogger {
    async fn publish(&self, event: EventEntity, _is_dlq: bool) -> Result<Id, IntegrationOSError> {
        tracing::info!("Received event: {:?}, using logger handler", event);

        Ok(Id::now(IdPrefix::PipelineEvent))
    }

    async fn consume(
        &self,
        _token: CancellationToken,
        _ctx: &AppState,
    ) -> Result<Unit, IntegrationOSError> {
        tracing::info!("Consuming events using logger handler");

        Ok(())
    }

    async fn process(&self, _ctx: &AppState, _event: &Record) -> Result<Unit, IntegrationOSError> {
        tracing::info!("Processing events using logger handler");

        Ok(())
    }
}
