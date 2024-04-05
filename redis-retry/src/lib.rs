pub mod config;

use anyhow::{Context, Result};
use futures_util::FutureExt;
use redis::{
    aio::{ConnectionLike, ConnectionManager},
    Client, Pipeline, RedisFuture, Value,
};
use tracing::warn;

pub use crate::config::Config;
pub use redis::{AsyncCommands, LposOptions, RedisResult};

#[derive(Clone)]
pub struct Redis {
    client: Client,
    conn: ConnectionManager,
    retry_count: u64,
}

impl Redis {
    pub async fn new(config: &Config) -> Result<Self> {
        Self::new_with_retry_count(config, std::u64::MAX).await
    }

    pub async fn new_with_retry_count(config: &Config, retry_count: u64) -> Result<Self> {
        let client =
            Client::open(config.url.clone()).with_context(|| "Could not parse redis url")?;
        let conn = client
            .get_tokio_connection_manager()
            .await
            .with_context(|| "Could not connect to redis")?;

        Ok(Self {
            client,
            conn,
            retry_count,
        })
    }
}

impl ConnectionLike for Redis {
    fn req_packed_command<'a>(&'a mut self, cmd: &'a redis::Cmd) -> RedisFuture<'a, Value> {
        (async move {
            let mut retry_count = 0u64;
            loop {
                let res = self.conn.req_packed_command(cmd).await;
                if res.is_ok() || retry_count >= self.retry_count {
                    return res;
                }
                warn!("Redis failed command, retrying...");
                retry_count += 1;
            }
        })
        .boxed()
    }

    fn req_packed_commands<'a>(
        &'a mut self,
        cmd: &'a Pipeline,
        offset: usize,
        count: usize,
    ) -> RedisFuture<'a, Vec<Value>> {
        (async move {
            let mut retry_count = 0u64;
            loop {
                let res = self.conn.req_packed_commands(cmd, offset, count).await;
                if res.is_ok() || retry_count >= self.retry_count {
                    return res;
                }
                warn!("Redis failed command, retrying...");
                retry_count += 1;
            }
        })
        .boxed()
    }

    fn get_db(&self) -> i64 {
        self.client.get_connection_info().redis.db
    }
}
