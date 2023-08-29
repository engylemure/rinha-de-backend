use opentelemetry::{
    global,
    runtime::TokioCurrentThread,
    sdk::propagation::TraceContextPropagator,
    sdk::{
        trace::{self, RandomIdGenerator, Sampler},
        Resource,
    },
    KeyValue,
};
use opentelemetry_otlp::WithExportConfig;
use std::str::FromStr;

use tracing_bunyan_formatter::{BunyanFormattingLayer, JsonStorageLayer};
use tracing_subscriber::{fmt::format::FmtSpan, layer::SubscriberExt, EnvFilter, Registry};

use super::env::EnvironmentValues;

pub fn init(env_values: &EnvironmentValues) -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::from_str(&env_values.rust_log)?)
        .with_span_events(FmtSpan::NEW | FmtSpan::CLOSE)
        .init();
    Ok(())
}

pub fn init_otel() {
    let app_name = "rinha-de-backend";

    // Start a new Jaeger trace pipeline.
    // Spans are exported in batch - recommended setup for a production application.
    global::set_text_map_propagator(TraceContextPropagator::new());
    let tracer = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(opentelemetry_otlp::new_exporter().tonic().with_env())
        .with_trace_config(
            trace::config()
                .with_sampler(Sampler::AlwaysOn)
                .with_id_generator(RandomIdGenerator::default())
                .with_resource(Resource::new(vec![KeyValue::new("service.name", app_name)])),
        )
        .install_batch(TokioCurrentThread)
        .expect("Failed to install OpenTelemetry tracer.");

    // Filter based on level - trace, debug, info, warn, error
    // Tunable via `RUST_LOG` env variable
    let env_filter = EnvFilter::try_from_default_env().unwrap_or(EnvFilter::new("info"));
    // Create a `tracing` layer using the Jaeger tracer
    let telemetry = tracing_opentelemetry::layer().with_tracer(tracer);
    // Create a `tracing` layer to emit spans as structured logs to stdout
    let formatting_layer = BunyanFormattingLayer::new(app_name.into(), std::io::stdout);
    // Combined them all together in a `tracing` subscriber
    let subscriber = Registry::default()
        .with(env_filter)
        .with(telemetry)
        .with(JsonStorageLayer)
        .with(formatting_layer);
    tracing::subscriber::set_global_default(subscriber)
        .expect("Failed to install `tracing` subscriber.")
}
