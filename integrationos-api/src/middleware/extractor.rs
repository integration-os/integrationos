use crate::{metrics::Metric, server::AppState, too_many_requests};
use anyhow::{Context, Result};
use axum::{
    body::Body,
    extract::State,
    middleware::Next,
    response::{IntoResponse, Response},
    Extension,
};
use http::{HeaderName, Request};
use integrationos_cache::remote::RedisCache;
use integrationos_domain::event_access::EventAccess;
use redis::AsyncCommands;
use std::sync::Arc;
use tokio::sync::{
    mpsc::{channel, Sender},
    oneshot,
};
use tracing::warn;

#[derive(Debug, Clone)]
pub struct RateLimiter {
    tx: Sender<(Arc<str>, oneshot::Sender<u64>)>,
    key_header_name: HeaderName,
    limit_header_name: HeaderName,
    remaining_header_name: HeaderName,
    reset_header_name: HeaderName,
    metric_tx: Sender<Metric>,
}

impl RateLimiter {
    pub async fn new(state: Arc<AppState>) -> Result<Self> {
        if state.config.rate_limit_enabled {
            return Err(anyhow::anyhow!("Rate limiting is disabled"));
        };

        let mut redis = RedisCache::new(&state.config.redis_config)
            .await
            .with_context(|| "Could not connect to redis")?;

        let (tx, mut rx) = channel::<(Arc<str>, oneshot::Sender<u64>)>(1024);

        let throughput_key = state.config.redis_config.api_throughput_key.clone();

        tokio::spawn(async move {
            while let Some((id, tx)) = rx.recv().await {
                let count: u64 = redis
                    .inner
                    .hincr(&throughput_key, id.as_ref(), 1)
                    .await
                    .unwrap_or_default();
                let _ = tx.send(count);
            }
        });

        let key_header_name =
            HeaderName::from_lowercase(state.config.headers.connection_header.as_bytes()).unwrap();

        let limit_header_name =
            HeaderName::from_lowercase(state.config.headers.rate_limit_limit.as_bytes()).unwrap();

        let remaining_header_name =
            HeaderName::from_lowercase(state.config.headers.rate_limit_remaining.as_bytes())
                .unwrap();

        let reset_header_name =
            HeaderName::from_lowercase(state.config.headers.rate_limit_reset.as_bytes()).unwrap();

        Ok(RateLimiter {
            tx,
            metric_tx: state.metric_tx.clone(),
            key_header_name,
            limit_header_name,
            remaining_header_name,
            reset_header_name,
        })
    }

    pub async fn get_request_count(&self, id: Arc<str>) -> u64 {
        let (tx, rx) = oneshot::channel();
        match self.tx.send((id, tx)).await {
            Ok(()) => rx.await.unwrap_or_default(),
            Err(e) => {
                warn!("Could not send to redis task: {e}");
                0
            }
        }
    }
}

pub async fn rate_limit(
    Extension(event_access): Extension<Arc<EventAccess>>,
    State(state): State<Arc<RateLimiter>>,
    req: Request<Body>,
    next: Next,
) -> Result<Response, Response> {
    let throughput = event_access.throughput;

    let count = state
        .get_request_count(event_access.ownership.id.clone())
        .await;

    if count >= throughput {
        let _ = state
            .metric_tx
            .send(Metric::rate_limited(
                event_access.clone(),
                req.headers().get(&state.key_header_name).cloned(),
            ))
            .await;
        let mut res = too_many_requests!().into_response();

        let headers = res.headers_mut();

        headers.insert(state.limit_header_name.clone(), throughput.into());
        headers.insert(state.remaining_header_name.clone(), 0.into());
        headers.insert(state.reset_header_name.clone(), 60.into());

        Err(res)
    } else {
        let mut res = next.run(req).await;
        let headers = res.headers_mut();

        headers.insert(state.limit_header_name.clone(), throughput.into());
        headers.insert(
            state.remaining_header_name.clone(),
            (throughput - count).into(),
        );
        headers.insert(state.reset_header_name.clone(), 60.into());
        Ok(res)
    }
}
