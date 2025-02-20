use std::{env, time::Duration};

use axum::{extract::MatchedPath, response::Response};
use gethostname::gethostname;
use http::{Request, Version, header};
use opentelemetry::{KeyValue, trace::SpanKind};
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::{
    Resource, runtime,
    trace::{self as sdktrace, BatchConfig},
};
use tonic::metadata::MetadataMap;
use tower_http::{
    classify::{ServerErrorsAsFailures, ServerErrorsFailureClass, SharedClassifier},
    trace::{MakeSpan, OnBodyChunk, OnEos, OnFailure, OnRequest, OnResponse, TraceLayer},
};
use tracing::{Span, debug, field::Empty, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

const QUEUE_SIZE: usize = 65_536;

pub fn tracing_init(service_name: &str, service_version: &str) {
    let mut exporter = opentelemetry_otlp::new_exporter()
        .tonic()
        .with_timeout(Duration::from_hours(24));

    if let Ok(apikey) = env::var("HONEYCOMB_APIKEY") {
        let mut headers = MetadataMap::new();
        headers.insert("x-honeycomb-team", apikey.parse().unwrap());
        exporter = exporter
            .with_endpoint("https://api.honeycomb.io:443")
            .with_metadata(headers);
    }

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
                    .with_exporter(exporter)
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
                            env!("RUSTC_VERSION"),
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

    if env::var("HONEYCOMB_APIKEY")
        .unwrap_or(String::from(""))
        .is_empty()
    {
        warn!("HONEYCOMB_APIKEY not set. Exporting to localhost");
    }
}

pub fn otel_tracing() -> TraceLayer<
    SharedClassifier<ServerErrorsAsFailures>,
    OtelMakeSpan,
    OtelOnRequest,
    OtelOnResponse,
    OtelOnBodyChunk,
    OtelOnEos,
    OtelOnFailure,
> {
    TraceLayer::new_for_http()
        .make_span_with(OtelMakeSpan)
        .on_request(OtelOnRequest)
        .on_response(OtelOnResponse)
        .on_body_chunk(OtelOnBodyChunk)
        .on_eos(OtelOnEos)
        .on_failure(OtelOnFailure)
}

#[derive(Clone, Copy, Debug)]
pub struct OtelMakeSpan;
impl<B> MakeSpan<B> for OtelMakeSpan {
    fn make_span(&mut self, request: &Request<B>) -> Span {
        let matched_route = request
            .extensions()
            .get::<MatchedPath>()
            .map(MatchedPath::as_str);

        tracing::info_span!(
            "http_request",
            "otel.name" = format!("{} {}", request.method(), matched_route.unwrap_or("UnknownRoute")),
            "otel.kind" = format!("{:?}", SpanKind::Server),
            "error.type" = Empty,

            "http.flavor" = match request.version() {
                Version::HTTP_09 => "0.9",
                Version::HTTP_10 => "1.0",
                Version::HTTP_11 => "1.1",
                Version::HTTP_2 => "2.0",
                Version::HTTP_3 => "3.0",
                _ => "Unknown",
            },
            "http.host" = request.headers() .get(header::HOST).map_or("", |h| h.to_str().unwrap_or("")),
            "http.request.content_length" = request.headers().get(header::CONTENT_LENGTH).and_then(|val| val.to_str().ok()),
            "http.request.method" = ?request.method(),
            "http.response.status_code" = Empty,
            "http.route" = matched_route,

            "url.path" = request.uri().path(),
            "url.query" = request.uri().query(),
            "url.scheme" = request.uri().scheme().map_or("http".to_string(), |s| s.to_string()),
            "user_agent.original" = request.headers().get(header::USER_AGENT).map_or("", |h| h.to_str().unwrap_or("")),

            "user.id" = Empty,
        )
    }
}

#[derive(Clone, Copy, Debug)]
pub struct OtelOnRequest;
impl<B> OnRequest<B> for OtelOnRequest {
    fn on_request(&mut self, _: &Request<B>, _: &Span) {}
}

#[derive(Clone, Copy, Debug)]
pub struct OtelOnResponse;
impl<B> OnResponse<B> for OtelOnResponse {
    fn on_response(self, response: &Response<B>, _: Duration, span: &Span) {
        span.record(
            "http.response.status_code",
            response.status().as_u16() as i64,
        );
        if response.status().is_server_error() {
            span.record("otel.status_code", "ERROR");
        }
        debug!("Request served");
    }
}

#[derive(Clone, Copy, Debug)]
pub struct OtelOnBodyChunk;
impl<B> OnBodyChunk<B> for OtelOnBodyChunk {
    fn on_body_chunk(&mut self, _: &B, _: Duration, _: &Span) {}
}

#[derive(Clone, Copy, Debug)]
pub struct OtelOnEos;
impl OnEos for OtelOnEos {
    fn on_eos(self, _: Option<&http::HeaderMap>, _: Duration, _: &Span) {}
}

#[derive(Clone, Copy, Debug)]
pub struct OtelOnFailure;
impl OnFailure<ServerErrorsFailureClass> for OtelOnFailure {
    fn on_failure(&mut self, error: ServerErrorsFailureClass, _: Duration, span: &Span) {
        span.record("otel.status_code", "ERROR");
        span.record("error.type", error.to_string());
        debug!("Request errored");
    }
}
