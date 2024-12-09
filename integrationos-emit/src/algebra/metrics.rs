use axum_prometheus::metrics::{counter, histogram, Counter, Histogram};
use axum_prometheus::{
    metrics_exporter_prometheus::{Matcher, PrometheusBuilder, PrometheusHandle},
    utils::SECONDS_DURATION_BUCKETS,
    GenericMetricLayer, Handle, PrometheusMetricLayerBuilder, AXUM_HTTP_REQUESTS_DURATION_SECONDS,
};
use integrationos_domain::Unit;
use std::time::Duration;

pub const EVENT_DURATION_KEY: &str = "event_duration_seconds";
pub const EVENT_COUNT_KEY: &str = "event_count";
pub const EVENT_ERRORS_KEY: &str = "event_errors";
pub const EVENT_SUCCESS_KEY: &str = "event_success";

pub const DLQ_DURATION_KEY: &str = "dlq_duration_seconds";
pub const DLQ_COUNT_KEY: &str = "dlq_count";
pub const DLQ_ERRORS_KEY: &str = "dlq_errors";
pub const DLQ_SUCCESS_KEY: &str = "dlq_success";

pub type MetricHandle = (
    GenericMetricLayer<'static, PrometheusHandle, Handle>,
    PrometheusHandle,
);

pub trait MetricExt<Exporter> {
    fn succeeded(&self, value: u64) -> Unit;
    fn errored(&self, value: u64) -> Unit;
    fn duration(&self, value: Duration) -> Unit;
}

pub struct MetricsRegistry {
    pub event_count: Counter,
    pub event_errors: Counter,
    pub event_success: Counter,
    pub event_duration: Histogram,
}

impl MetricsRegistry {
    pub fn noop() -> Self {
        Self {
            event_count: Counter::noop(),
            event_errors: Counter::noop(),
            event_success: Counter::noop(),
            event_duration: Histogram::noop(),
        }
    }

    pub fn handle() -> MetricHandle {
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
            .build_pair()
    }
}

impl Default for MetricsRegistry {
    fn default() -> Self {
        Self {
            event_count: counter!(EVENT_COUNT_KEY, "events" => "count"),
            event_errors: counter!(EVENT_ERRORS_KEY, "events" => "errors"),
            event_success: counter!(EVENT_SUCCESS_KEY, "events" => "success"),
            event_duration: histogram!(EVENT_DURATION_KEY, "events" => "duration"),
            // dlq_count: counter!(DLQ_COUNT_KEY, "dlq" => "count"),
            // dlq_errors: counter!(DLQ_ERRORS_KEY, "dlq" => "errors"),
            // dlq_success: counter!(DLQ_SUCCESS_KEY, "dlq" => "success"),
        }
    }
}

impl MetricExt<MetricHandle> for MetricsRegistry {
    fn succeeded(&self, value: u64) -> Unit {
        self.event_success.increment(value);
        self.event_count.increment(value);
    }

    fn errored(&self, value: u64) -> Unit {
        self.event_errors.increment(value);
        self.event_count.increment(value);
    }

    fn duration(&self, value: Duration) -> Unit {
        self.event_duration.record(value);
    }
}
