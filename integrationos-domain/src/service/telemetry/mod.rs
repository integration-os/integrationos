use crate::TimedExt;
use axum::body::Body;
use axum::extract::Request;
use axum::middleware::Next;
use axum::response::IntoResponse;
use http::StatusCode;
use opentelemetry::{global, trace::TracerProvider};
use opentelemetry_otlp::{SpanExporter, WithExportConfig};
use opentelemetry_sdk::runtime::Tokio;
use opentelemetry_sdk::trace::TracerProvider as OtelTracerProvider;
use tracing::subscriber::set_global_default;
use tracing_bunyan_formatter::{BunyanFormattingLayer, JsonStorageLayer};
use tracing_log::LogTracer;
use tracing_opentelemetry::OpenTelemetryLayer;
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
    otlp_url: Option<String>,
) -> Telemetry<impl SubscriberExt + Send + Sync + 'static>
where
    Sink: for<'a> MakeWriter<'a> + Send + Sync + 'static,
{
    let formatting_layer: BunyanFormattingLayer<Sink> =
        BunyanFormattingLayer::new(name.clone(), sink);

    let filter_layer =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(env_filter));

    match otlp_url {
        Some(otlp_url) => {
            let exporter = SpanExporter::builder()
                .with_tonic()
                .with_endpoint(otlp_url.clone())
                .build()
                .expect("Failed to create OTLP exporter");

            let provider = OtelTracerProvider::builder()
                .with_resource(opentelemetry_sdk::Resource::new(vec![
                    opentelemetry::KeyValue::new("service.name", name),
                ]))
                .with_batch_exporter(exporter, Tokio)
                .build();

            global::set_tracer_provider(provider.clone());
            let tracer = provider.tracer("tracing-otel-subscriber");

            let telemetry: Telemetry<Box<dyn SubscriberExt + Send + Sync>> = Telemetry {
                subscriber: Box::new(
                    Registry::default()
                        .with(OpenTelemetryLayer::new(tracer))
                        .with(filter_layer)
                        .with(JsonStorageLayer)
                        .with(formatting_layer),
                ),
            };

            telemetry
        }
        None => {
            let telemetry: Telemetry<Box<dyn SubscriberExt + Send + Sync>> = Telemetry {
                subscriber: Box::new(
                    Registry::default()
                        .with(filter_layer)
                        .with(JsonStorageLayer)
                        .with(formatting_layer),
                ),
            };
            telemetry
        }
    }
}

pub fn init_subscriber(subscriber: Telemetry<impl SubscriberExt + Send + Sync + 'static>) {
    LogTracer::init().expect("Failed to set logger");
    set_global_default(subscriber.subscriber).expect("Failed to set subscriber");
}

pub async fn log_request_middleware(
    req: Request<Body>,
    next: Next,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let path = req.uri().path().to_string();
    let method = req.method().to_string();
    let res = next
        .run(req)
        .timed(|response, elapsed| {
            let status = response.status();
            let logger = |str| {
                if status.is_server_error() {
                    tracing::error!("{}", str)
                } else if status.is_client_error() {
                    tracing::warn!("{}", str)
                } else {
                    tracing::info!("{}", str)
                }
            };

            logger(format!(
                "[{} {}] Elapsed time: {}ms | Status: {}",
                method,
                path,
                elapsed.as_millis(),
                status,
            ));
        })
        .await;

    Ok(res)
}

#[derive(Debug)]
pub struct OtelGuard {
    pub otlp_url: Option<String>,
}

impl Drop for OtelGuard {
    fn drop(&mut self) {
        if self.otlp_url.is_some() {
            tracing::info!("Shutting down OpenTelemetry");
            opentelemetry::global::shutdown_tracer_provider();
        }
    }
}
