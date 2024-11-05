use super::EventStreamExt;
use crate::domain::{
    config::{EmitterConfig, StreamConfig},
    event::{Event, EventEntity},
};
use anyhow::Context;
use async_trait::async_trait;
use fluvio::{
    consumer::{
        ConsumerConfigExt, ConsumerConfigExtBuilder, ConsumerStream, OffsetManagementStrategy,
    },
    spu::SpuSocketPool,
    Compression, Fluvio, FluvioConfig, Offset, RetryPolicy, TopicProducer,
    TopicProducerConfigBuilder,
};
use futures::StreamExt;
use integrationos_domain::{prefix::IdPrefix, Id, IntegrationOSError, InternalError, Unit};
use std::boxed::Box;
use std::time::Duration;
use tokio_util::sync::CancellationToken;

pub struct ConsumerConfig {
    ext: ConsumerConfigExt,
    app: StreamConfig,
}

pub struct FluvioDriverImpl {
    pub client: Fluvio,
    pub consumer_conf: Option<ConsumerConfig>,
    pub producer: Option<TopicProducer<SpuSocketPool>>,
}

impl FluvioDriverImpl {
    pub async fn new(config: &EmitterConfig) -> Result<Self, IntegrationOSError> {
        let fluvio_config = FluvioConfig::new(config.fluvio.endpoint());
        let fluvio_client = Fluvio::connect_with_config(&fluvio_config).await?;

        let producer = match &config.fluvio.producer_topic {
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

                Some(
                    fluvio_client
                        // TODO: Use topic producer with config
                        .topic_producer_with_config(producer_topic, config)
                        .await?,
                )
            }
            None => None,
        };

        let consumer = match &config.fluvio.consumer_topic {
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

                Some(ConsumerConfig {
                    ext,
                    app: config.fluvio.clone(),
                })
            }
            None => None,
        };

        Ok(Self {
            client: fluvio_client,
            consumer_conf: consumer,
            producer,
        })
    }
}

#[async_trait]
impl EventStreamExt for FluvioDriverImpl {
    async fn publish(&self, event: EventEntity) -> Result<Id, IntegrationOSError> {
        match &self.producer {
            Some(producer) => {
                let payload = serde_json::to_vec(&event).map_err(|e| {
                    InternalError::serialize_error(&format!("Could not serialize event: {e}"), None)
                })?;

                producer
                    .send(event.partition_key(), payload)
                    .await
                    .map_err(|e| {
                        InternalError::io_err(&format!("Could not send event to fluvio: {e}"), None)
                    })?;

                Ok(event.entity_id)
            }
            None => Err(InternalError::invalid_argument(
                "Producer not initialized",
                None,
            )),
        }
    }

    async fn consume(&self, token: CancellationToken) -> Result<Unit, IntegrationOSError> {
        match &self.consumer_conf {
            None => Err(InternalError::invalid_argument(
                "Consumer not initialized",
                None,
            )),
            Some(consumer_conf) => {
                let mut stream = self
                    .client
                    .consumer_with_config(consumer_conf.ext.clone())
                    .await?;
                let mut count = 0;
                let mut interval = tokio::time::interval(Duration::from_millis(
                    consumer_conf.app.consumer_linger_time,
                ));
                interval.tick().await;

                loop {
                    tokio::select! {
                        _ = token.cancelled() => {
                            tracing::info!("Consumer cancelled, gracefully shutting down");
                            stream.offset_commit().map_err(|err| anyhow::anyhow!(err))?;
                            stream.offset_flush().await.map_err(|err| anyhow::anyhow!(err))?;
                            return Ok(());
                        },
                        timeout = interval.tick() => {
                            if count > 0 {
                                tracing::info!("Committing offsets after {timeout:?}");
                                stream.offset_commit().map_err(|err| anyhow::anyhow!(err))?;
                                stream.offset_flush().await.map_err(|err| anyhow::anyhow!(err))?;
                                tracing::info!("Periodic offset commit completed.");
                                count = 0; // Reset count after commit
                            }
                        },
                        record = stream.next() => {
                            count += 1;
                            match record {
                                Some(Ok(record)) => tracing::info!("Consumed record: {}", record.get_value().as_utf8_lossy_string()),
                                Some(Err(err)) => tracing::error!("Error consuming record: {err}"),
                                None => tracing::info!("Consumer stream closed")
                            }

                            if count >= consumer_conf.app.consumer_batch_size {
                                count = 0;
                                stream.offset_commit().map_err(|err| anyhow::anyhow!(err))?;
                                stream.offset_flush().await.map_err(|err| anyhow::anyhow!(err))?;
                            }
                        }
                    };
                }
            }
        }
    }
}

pub struct FluvioDriverLogger;

#[async_trait]
impl EventStreamExt for FluvioDriverLogger {
    async fn publish(&self, event: EventEntity) -> Result<Id, IntegrationOSError> {
        tracing::info!("Received event: {:?}, using logger handler", event);

        Ok(Id::now(IdPrefix::PipelineEvent))
    }

    async fn consume(&self, _token: CancellationToken) -> Result<Unit, IntegrationOSError> {
        tracing::info!("Consuming events using logger handler");

        Ok(())
    }
}
