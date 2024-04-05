use crate::store::{ContextStore, ControlDataStore};
use anyhow::{Context, Result};
use integrationos_domain::common::{event_with_context::EventWithContext, Event, Transaction};
use redis_retry::{AsyncCommands, Config, Redis};
use std::{sync::Arc, time::Duration};
use tokio::{join, sync::Mutex, time::sleep};
use tracing::error;

#[derive(Clone)]
pub struct EventHandler<
    T: ControlDataStore + Sync + Send + 'static,
    U: ContextStore + Sync + Send + 'static,
> {
    config: Config,
    redis: Arc<Mutex<Redis>>,
    control_store: Arc<T>,
    context_store: Arc<U>,
}

impl<T: ControlDataStore + Sync + Send + 'static, U: ContextStore + Sync + Send + 'static>
    EventHandler<T, U>
{
    pub async fn new(config: Config, control_store: Arc<T>, context_store: Arc<U>) -> Result<Self> {
        let redis = Arc::new(Mutex::new(Redis::new(&config).await?));

        Ok(Self {
            config,
            redis,
            control_store,
            context_store,
        })
    }

    pub async fn pop_event(&self) -> Result<EventWithContext> {
        loop {
            {
                if let Some(data) = async {
                    let mut conn = self.redis.lock().await;
                    conn.rpop::<&str, Option<Vec<u8>>>(&self.config.queue_name, None)
                        .await
                        .with_context(|| "failed to parse redis message")
                }
                .await?
                {
                    let event: EventWithContext = serde_json::from_slice(&data)?;
                    return Ok(event);
                }
            }
            sleep(Duration::from_millis(50)).await;
        }
    }

    pub async fn increment_throughput_count(&self, event: &Event) -> Result<bool> {
        let connection = self
            .control_store
            .fetch_connection(event)
            .await
            .with_context(|| "Could not fetch integration")?;
        let throughput = connection.throughput;

        let count: u64 = async {
            let mut conn = self.redis.lock().await;
            conn.hincr(&self.config.event_throughput_key, &throughput.key, 1)
                .await
                .with_context(|| "Could not increment throughput for integration")
        }
        .await?;

        Ok(count <= throughput.limit)
    }

    pub async fn defer_event(&self, mut event: EventWithContext) -> Result<()> {
        let count = if let Some(transaction) = event.context.transaction {
            if let Some(Ok(number)) = transaction
                .tx_key
                .split("::throttled-")
                .last()
                .map(|n| n.parse::<u64>())
            {
                number + 1
            } else {
                1
            }
        } else {
            1
        };
        event.context.transaction = Some(Transaction::throttled(
            &event.event,
            format!("{}::throttled-{count}", event.event.key),
            "".to_owned(),
            "".to_owned(),
        ));
        let context_fut = self.context_store.set(event.context.clone());
        let redis_fut = async {
            let serialized = serde_json::to_vec(&event)
                .with_context(|| "Could not serialize event with context")?;
            let mut conn = self.redis.lock().await;

            conn.lpush::<&str, &[u8], ()>(&self.config.queue_name, &serialized)
                .await
                .with_context(|| "Could not send channel response to queue")
        };
        let (context_res, redis_res) = join!(context_fut, redis_fut);
        if let Err(e) = context_res {
            error!("Could not write throttle context to context store: {e}");
        }
        redis_res
    }
}
