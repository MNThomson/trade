use std::{env, time::Duration};

use gethostname::gethostname;
use opentelemetry::KeyValue;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::{
    Resource, runtime,
    trace::{self as sdktrace, BatchConfig},
};
use rustc_version::version;
use tonic::metadata::MetadataMap;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

const QUEUE_SIZE: usize = 65_536;

pub fn tracing_init(service_name: &str, service_version: &str) {
    #[cfg(not(debug_assertions))]
    env::var("HONEYCOMB_APIKEY").expect("HONEYCOMB_APIKEY not set");

    let apikey = env::var("HONEYCOMB_APIKEY").unwrap_or("".to_string());

    let mut headers = MetadataMap::new();
    headers.insert("x-honeycomb-team", apikey.parse().unwrap());

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                format!(
                    "none,{}=debug,{}=debug,axum::rejection=trace",
                    service_name,
                    env!("CARGO_PKG_NAME")
                )
                .into()
            }),
        )
        .with(tracing_subscriber::fmt::layer())
        .with(
            tracing_opentelemetry::layer().with_tracer(
                opentelemetry_otlp::new_pipeline()
                    .tracing()
                    .with_exporter(
                        opentelemetry_otlp::new_exporter()
                            .tonic()
                            .with_endpoint("https://api.honeycomb.io:443")
                            .with_metadata(headers),
                    )
                    .with_batch_config(
                        BatchConfig::default()
                            .with_max_queue_size(QUEUE_SIZE)
                            .with_max_concurrent_exports(2)
                            .with_max_export_timeout(Duration::from_secs(5)),
                    )
                    .with_trace_config(sdktrace::config().with_resource(Resource::new(vec![
                        KeyValue::new("service.name", service_name.to_string()),
                        KeyValue::new("service.version", service_version.to_string()),
                        KeyValue::new("process.runtime.name", "rustc"),
                        KeyValue::new(
                            "process.runtime.version",
                            version().unwrap().to_string(),
                        ),
                        KeyValue::new("process.command", std::env::args().next().unwrap()),
                        KeyValue::new(
                            "process.command_line",
                            std::env::args().collect::<Vec<_>>().join(" "),
                        ),
                        KeyValue::new(
                            "process.executable.name",
                            std::env::current_exe()
                                .unwrap()
                                .file_name()
                                .unwrap()
                                .to_string_lossy()
                                .into_owned(),
                        ),
                        KeyValue::new(
                            "process.executable.path",
                            std::env::current_exe()
                                .unwrap()
                                .display()
                                .to_string(),
                        ),
                        KeyValue::new("process.pid", std::process::id() as i64),
                        KeyValue::new("host.arch", std::env::consts::ARCH),
                        KeyValue::new("host.name", gethostname().into_string().unwrap()),
                    ])))
                    .install_batch(runtime::Tokio)
                    .unwrap(),
            ),
        )
        .init();
}
