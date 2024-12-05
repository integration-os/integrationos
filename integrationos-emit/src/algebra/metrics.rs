use crate::{domain::metrics::MetricHandle, stream::EventStreamTopic};
use axum_prometheus::metrics::{counter, histogram, Counter, Histogram};
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

pub trait MetricExt<Exporter> {
    fn succeeded(&self, value: u64, topic: EventStreamTopic) -> Unit;
    fn errored(&self, value: u64, topic: EventStreamTopic) -> Unit;
    fn duration(&self, value: Duration) -> Unit;
}

pub struct MetricsRegistry {
    event_count: Counter,
    event_errors: Counter,
    event_success: Counter,
    event_duration: Histogram,
    dlq_count: Counter,
    dlq_errors: Counter,
    dlq_success: Counter,
}

impl Default for MetricsRegistry {
    fn default() -> Self {
        Self {
            event_count: counter!(EVENT_COUNT_KEY, "events" => "count"),
            event_errors: counter!(EVENT_ERRORS_KEY, "events" => "errors"),
            event_success: counter!(EVENT_SUCCESS_KEY, "events" => "success"),
            event_duration: histogram!(EVENT_DURATION_KEY, "events" => "duration"),
            dlq_count: counter!(DLQ_COUNT_KEY, "dlq" => "count"),
            dlq_errors: counter!(DLQ_ERRORS_KEY, "dlq" => "errors"),
            dlq_success: counter!(DLQ_SUCCESS_KEY, "dlq" => "success"),
        }
    }
}

impl MetricExt<MetricHandle> for MetricsRegistry {
    fn succeeded(&self, value: u64, topic: EventStreamTopic) -> Unit {
        match topic {
            EventStreamTopic::Target => {
                self.event_success.increment(value);
                self.event_count.increment(value);
            }
            EventStreamTopic::Dlq => {
                self.dlq_success.increment(value);
                self.dlq_count.increment(value);
            }
        }
    }

    fn errored(&self, value: u64, topic: EventStreamTopic) -> Unit {
        match topic {
            EventStreamTopic::Target => {
                self.event_errors.increment(value);
                self.event_count.increment(value);
            }
            EventStreamTopic::Dlq => {
                self.dlq_errors.increment(value);
                self.dlq_count.increment(value);
            }
        }
    }

    fn duration(&self, value: Duration) -> Unit {
        self.event_duration.record(value);
    }
}
