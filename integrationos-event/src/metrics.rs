// number of events currently being processed
pub const CONCURRENT_EVENTS_GAUGE: &str = "concurrent_events";
pub const CONCURRENT_EVENTS_PERCENTAGE_GAUGE: &str = "concurrent_events_percentage";
// counter of total events processed, bucketed into successful or dropped
pub const EVENTS_HISTOGRAM: &str = "events_processed";
pub const STATUS_LABEL: &str = "status";
// histogram of elapsed time per stage
pub const STAGE_HISTOGRAM: &str = "stages_processed";
pub const STAGE_LABEL: &str = "stage";
