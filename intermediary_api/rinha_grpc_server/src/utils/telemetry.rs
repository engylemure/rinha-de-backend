use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

pub fn init() {
    tracing_subscriber::registry()
        .with(
            fmt::layer().with_span_events(fmt::format::FmtSpan::NEW | fmt::format::FmtSpan::CLOSE),
        )
        .with(EnvFilter::from_default_env())
        .init();
}

pub fn init_otel() {
    init_tracing_opentelemetry::tracing_subscriber_ext::init_subscribers()
        .expect("init subscribers");
}
