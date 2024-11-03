use super::EventStreamExt;
use crate::domain::{config::EmitterConfig, event::EventEntity};
use async_trait::async_trait;
use fluvio::{
    spu::SpuSocketPool, Compression, Fluvio, FluvioConfig, RetryPolicy, TopicProducer,
    TopicProducerConfigBuilder,
};
use integrationos_domain::{prefix::IdPrefix, Id, IntegrationOSError, InternalError};
use std::time::Duration;

pub struct FluvioDriverImpl {
    pub client: Fluvio,
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
                    .batch_size(config.fluvio.batch_size)
                    .linger(Duration::from_millis(config.fluvio.linger_time))
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

        Ok(Self {
            client: fluvio_client,
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
}

pub struct FluvioDriverLogger;

#[async_trait]
impl EventStreamExt for FluvioDriverLogger {
    async fn publish(&self, event: EventEntity) -> Result<Id, IntegrationOSError> {
        tracing::info!("Received event: {:?}, using logger handler", event);

        Ok(Id::now(IdPrefix::PipelineEvent))
    }
}
