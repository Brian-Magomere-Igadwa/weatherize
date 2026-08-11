use opentelemetry::global;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::trace::Tracer;
use tracing_bunyan_formatter::{BunyanFormattingLayer, JsonStorageLayer};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::{EnvFilter, Registry};

pub fn init_telemetry(service_name: &str, otlp_endpoint: &str) -> Tracer {
    let tracer = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(
            opentelemetry_otlp::new_exporter()
                .tonic()
                .with_endpoint(otlp_endpoint),
        )
        .with_trace_config(opentelemetry_sdk::trace::config().with_resource(
            opentelemetry_sdk::Resource::new(vec![opentelemetry::KeyValue::new(
                "service.name",
                service_name.to_string(),
            )]),
        ))
        .install_batch(opentelemetry_sdk::runtime::Tokio)
        .expect("Failed to initialize OpenTelemetry OTLP tracer");

    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let telemetry_layer = tracing_opentelemetry::layer().with_tracer(tracer.clone());
    let formatting_layer = BunyanFormattingLayer::new(service_name.to_string(), std::io::stdout);

    let subscriber = Registry::default()
        .with(env_filter)
        .with(JsonStorageLayer)
        .with(formatting_layer)
        .with(telemetry_layer);

    tracing::subscriber::set_global_default(subscriber)
        .expect("Failed to set global tracing subscriber");

    tracer
}

pub fn shutdown_telemetry() {
    global::shutdown_tracer_provider();
}
