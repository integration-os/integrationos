use crate::algebra::metrics::{DLQ_DURATION_KEY, EVENT_DURATION_KEY};
use axum_prometheus::{
    metrics_exporter_prometheus::{Matcher, PrometheusBuilder, PrometheusHandle},
    utils::SECONDS_DURATION_BUCKETS,
    GenericMetricLayer, Handle, PrometheusMetricLayerBuilder, AXUM_HTTP_REQUESTS_DURATION_SECONDS,
};

pub type MetricHandle = (
    GenericMetricLayer<'static, PrometheusHandle, Handle>,
    PrometheusHandle,
);

#[derive(Clone)]
pub struct MetricsLayer {
    pub exporter: Option<MetricHandle>,
}

impl MetricsLayer {
    pub fn noop() -> Self {
        Self { exporter: None }
    }
}

impl Default for MetricsLayer {
    fn default() -> Self {
        MetricsLayer {
            exporter: Some(
                PrometheusMetricLayerBuilder::new()
                    .with_metrics_from_fn(|| {
                        PrometheusBuilder::new()
                            .set_buckets_for_metric(
                                Matcher::Full(AXUM_HTTP_REQUESTS_DURATION_SECONDS.to_string()),
                                SECONDS_DURATION_BUCKETS,
                            )
                            .expect("Unable to install request matcher")
                            .set_buckets_for_metric(
                                Matcher::Full(EVENT_DURATION_KEY.to_string()),
                                SECONDS_DURATION_BUCKETS,
                            )
                            .expect("Unable to install event recorder matcher")
                            .set_buckets_for_metric(
                                Matcher::Full(DLQ_DURATION_KEY.to_string()),
                                SECONDS_DURATION_BUCKETS,
                            )
                            .expect("Unable to install dlq recorder matcher")
                            .install_recorder()
                            .expect("Unable to setup metrics")
                    })
                    .with_ignore_pattern("/metrics")
                    .build_pair(),
            ),
        }
    }
}
