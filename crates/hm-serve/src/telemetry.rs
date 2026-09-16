#![allow(clippy::missing_errors_doc)]

use hm_core::telemetry::{Attribute, SpanKind, SpanOutcome, SpanRecord, SpanSink};
use hm_core::{Error, ErrorCode};
use serde_json::{Value, json};
use std::fs::{File, OpenOptions};
use std::io::Write as _;
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock, PoisonError};

const MAXIMUM_BUFFERED_SPANS: usize = 1024;
const SCOPE_NAME: &str = "hypermind";
const DEFAULT_SERVICE: &str = "hypermind";

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum TelemetryMode {
    #[default]
    Off,
    File,
}

struct Destination {
    path: PathBuf,
    service: String,
}

#[derive(Default)]
struct SinkState {
    destination: Option<Destination>,
    pending: Vec<String>,
}

#[derive(Default)]
pub struct FileSpanSink {
    state: Mutex<SinkState>,
}

impl FileSpanSink {
    fn locked(&self) -> MutexGuard<'_, SinkState> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn point(&self, destination: Option<Destination>) {
        self.write_pending();
        self.locked().destination = destination;
    }

    fn write_pending(&self) {
        let mut state = self.locked();
        if state.pending.is_empty() {
            return;
        }
        let Some(path) = state
            .destination
            .as_ref()
            .map(|destination| destination.path.clone())
        else {
            state.pending.clear();
            return;
        };
        let lines = std::mem::take(&mut state.pending);
        let _ = append_lines(&path, &lines);
    }
}

impl SpanSink for FileSpanSink {
    fn enabled(&self) -> bool {
        self.locked().destination.is_some()
    }

    fn record(&self, span: &SpanRecord) {
        let mut state = self.locked();
        let Some(service) = state
            .destination
            .as_ref()
            .map(|destination| destination.service.clone())
        else {
            return;
        };
        state.pending.push(encode(span, &service));
        let full = state.pending.len() >= MAXIMUM_BUFFERED_SPANS;
        drop(state);
        if full {
            self.write_pending();
        }
    }

    fn flush(&self) {
        self.write_pending();
    }
}

pub fn configure(mode: TelemetryMode, path: Option<&Path>, service: &str) -> Result<bool, Error> {
    match mode {
        TelemetryMode::Off => {
            if let Some(sink) = SINK.get() {
                sink.point(None);
            }
            Ok(false)
        }
        TelemetryMode::File => {
            let path = path.ok_or_else(|| Error::new(ErrorCode::InvalidArgument))?;
            if let Some(parent) = path
                .parent()
                .filter(|parent| !parent.as_os_str().is_empty())
                && !parent.is_dir()
            {
                return Err(Error::new(ErrorCode::OpenFailed));
            }
            drop(open_append(path)?);
            let service = if service.is_empty() {
                DEFAULT_SERVICE
            } else {
                service
            };
            sink().point(Some(Destination {
                path: path.to_path_buf(),
                service: service.to_owned(),
            }));
            Ok(true)
        }
    }
}

pub fn flush() {
    hm_core::telemetry::flush_spans();
}

static SINK: OnceLock<Arc<FileSpanSink>> = OnceLock::new();

fn sink() -> &'static Arc<FileSpanSink> {
    SINK.get_or_init(|| {
        let sink = Arc::new(FileSpanSink::default());
        let installed: Arc<dyn SpanSink> = sink.clone();
        let _ = hm_core::telemetry::install_span_sink(installed);
        sink
    })
}

fn open_append(path: &Path) -> Result<File, Error> {
    OpenOptions::new()
        .append(true)
        .create(true)
        .mode(0o600)
        .open(path)
        .map_err(|error| {
            Error::new(ErrorCode::OpenFailed).with_system_error(error.raw_os_error().unwrap_or(0))
        })
}

fn append_lines(path: &Path, lines: &[String]) -> Result<(), Error> {
    let mut buffer = String::new();
    for line in lines {
        buffer.push_str(line);
        buffer.push('\n');
    }
    let mut file = open_append(path)?;
    file.write_all(buffer.as_bytes()).map_err(|error| {
        Error::new(ErrorCode::WriteFailed).with_system_error(error.raw_os_error().unwrap_or(0))
    })?;
    file.sync_all().map_err(|error| {
        Error::new(ErrorCode::SyncFailed).with_system_error(error.raw_os_error().unwrap_or(0))
    })
}

fn encode(span: &SpanRecord, service: &str) -> String {
    let mut attributes = vec![json!({
        "key": "hypermind.span.kind",
        "value": {"stringValue": span.kind.as_str()},
    })];
    for attribute in &span.attributes {
        let (key, value) = match *attribute {
            Attribute::Integer(key, value) => (key, json!({"intValue": value})),
            Attribute::Text(key, value) => (key, json!({"stringValue": value})),
            Attribute::Boolean(key, value) => (key, json!({"boolValue": value})),
        };
        attributes.push(json!({"key": key, "value": value}));
    }
    let end_unix_nanos = span
        .start_unix_nanos
        .saturating_add(u128::from(span.duration_nanos.max(1)));
    let line: Value = json!({
        "resourceSpans": [{
            "resource": {
                "attributes": [{"key": "service.name", "value": {"stringValue": service}}],
            },
            "scopeSpans": [{
                "scope": {"name": SCOPE_NAME, "version": crate::VERSION},
                "spans": [{
                    "traceId": hex(&span.trace_id),
                    "spanId": hex(&span.span_id),
                    "name": span.name,
                    "kind": otlp_kind(span.kind),
                    "startTimeUnixNano": span.start_unix_nanos.to_string(),
                    "endTimeUnixNano": end_unix_nanos.to_string(),
                    "attributes": attributes,
                    "status": {"code": status_code(span.outcome)},
                }],
            }],
        }],
    });
    line.to_string()
}

fn hex(bytes: &[u8]) -> String {
    let mut encoded = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use std::fmt::Write as _;
        let _ = write!(encoded, "{byte:02x}");
    }
    encoded
}

const fn otlp_kind(kind: SpanKind) -> u8 {
    match kind {
        SpanKind::Request => 2,
        SpanKind::Provider => 3,
        SpanKind::Ingestion | SpanKind::Extraction => 1,
    }
}

const fn status_code(outcome: SpanOutcome) -> u8 {
    match outcome {
        SpanOutcome::Ok => 1,
        SpanOutcome::Error => 2,
    }
}
