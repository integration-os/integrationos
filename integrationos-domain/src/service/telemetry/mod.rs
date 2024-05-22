use tracing::subscriber::set_global_default;
use tracing_bunyan_formatter::{BunyanFormattingLayer, JsonStorageLayer};
use tracing_log::LogTracer;
use tracing_subscriber::fmt::MakeWriter;
use tracing_subscriber::{layer::SubscriberExt, EnvFilter, Registry};

pub struct Telemetry<T>
where
    T: SubscriberExt + Send + Sync + 'static,
{
    pub subscriber: T,
}

pub fn get_subscriber<Sink>(
    name: String,
    env_filter: String,
    sink: Sink,
) -> Telemetry<impl SubscriberExt + Send + Sync + 'static>
where
    Sink: for<'a> MakeWriter<'a> + Send + Sync + 'static,
{
    let formatting_layer: BunyanFormattingLayer<Sink> = BunyanFormattingLayer::new(name, sink);

    let filter_layer =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(env_filter));

    Telemetry {
        subscriber: Registry::default()
            .with(filter_layer)
            .with(JsonStorageLayer)
            .with(formatting_layer),
    }
}

pub fn init_subscriber(subscriber: Telemetry<impl SubscriberExt + Send + Sync + 'static>) {
    LogTracer::init().expect("Failed to set logger");
    set_global_default(subscriber.subscriber).expect("Failed to set subscriber");
}
