use sha2::{Digest, Sha256};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, OnceLock};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

const MAXIMUM_ATTRIBUTES: usize = 16;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum SpanKind {
    Request,
    Ingestion,
    Extraction,
    Provider,
}

impl SpanKind {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Request => "request",
            Self::Ingestion => "ingestion",
            Self::Extraction => "extraction",
            Self::Provider => "provider",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum SpanOutcome {
    Ok,
    Error,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Attribute {
    Integer(&'static str, i64),
    Text(&'static str, &'static str),
    Boolean(&'static str, bool),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SpanRecord {
    pub kind: SpanKind,
    pub name: &'static str,
    pub trace_id: [u8; 16],
    pub span_id: [u8; 8],
    pub start_unix_nanos: u128,
    pub duration_nanos: u64,
    pub outcome: SpanOutcome,
    pub attributes: Vec<Attribute>,
}

pub trait SpanSink: Send + Sync {
    fn enabled(&self) -> bool;
    fn record(&self, span: &SpanRecord);
    fn flush(&self);
}

pub struct SpanBuilder {
    kind: SpanKind,
    name: &'static str,
    trace_id: [u8; 16],
    span_id: [u8; 8],
    start_unix_nanos: u128,
    started: Instant,
    attributes: Vec<Attribute>,
}

impl SpanBuilder {
    pub fn attribute(&mut self, attribute: Attribute) {
        if self.attributes.len() < MAXIMUM_ATTRIBUTES {
            self.attributes.push(attribute);
        }
    }

    pub fn finish(self, outcome: SpanOutcome) {
        let Some(sink) = span_sink() else {
            return;
        };
        if !sink.enabled() {
            return;
        }
        sink.record(&SpanRecord {
            kind: self.kind,
            name: self.name,
            trace_id: self.trace_id,
            span_id: self.span_id,
            start_unix_nanos: self.start_unix_nanos,
            duration_nanos: u64::try_from(self.started.elapsed().as_nanos()).unwrap_or(u64::MAX),
            outcome,
            attributes: self.attributes,
        });
    }
}

static SINK: OnceLock<Arc<dyn SpanSink>> = OnceLock::new();
static SEED: OnceLock<[u8; 32]> = OnceLock::new();
static COUNTER: AtomicU64 = AtomicU64::new(0);

#[must_use]
pub fn install_span_sink(sink: Arc<dyn SpanSink>) -> bool {
    SINK.set(sink).is_ok()
}

#[must_use]
pub fn span_sink() -> Option<&'static Arc<dyn SpanSink>> {
    SINK.get()
}

#[must_use]
pub fn start_span(kind: SpanKind, name: &'static str) -> Option<SpanBuilder> {
    let sink = span_sink()?;
    if !sink.enabled() {
        return None;
    }
    let (trace_id, span_id) = identifiers();
    Some(SpanBuilder {
        kind,
        name,
        trace_id,
        span_id,
        start_unix_nanos: unix_nanos(),
        started: Instant::now(),
        attributes: Vec::new(),
    })
}

pub fn flush_spans() {
    if let Some(sink) = span_sink() {
        sink.flush();
    }
}

fn identifiers() -> ([u8; 16], [u8; 8]) {
    let counter = COUNTER.fetch_add(1, Ordering::Relaxed);
    let seed = SEED.get_or_init(|| {
        let mut hasher = Sha256::new();
        hasher.update(std::process::id().to_le_bytes());
        hasher.update(unix_nanos().to_le_bytes());
        hasher.finalize().into()
    });
    let mut hasher = Sha256::new();
    hasher.update(seed);
    hasher.update(counter.to_le_bytes());
    let digest: [u8; 32] = hasher.finalize().into();
    let mut trace_id = [0; 16];
    trace_id.copy_from_slice(&digest[..16]);
    let mut span_id = [0; 8];
    span_id.copy_from_slice(&digest[16..24]);
    if trace_id == [0; 16] {
        trace_id[15] = 1;
    }
    if span_id == [0; 8] {
        span_id[7] = 1;
    }
    (trace_id, span_id)
}

fn unix_nanos() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_nanos())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identifiers_are_unique_and_never_zero() {
        let (first_trace, first_span) = identifiers();
        let (second_trace, second_span) = identifiers();
        assert_ne!(first_trace, second_trace);
        assert_ne!(first_span, second_span);
        assert_ne!(first_trace, [0; 16]);
        assert_ne!(first_span, [0; 8]);
    }

    #[test]
    fn no_installed_sink_means_no_span() {
        assert!(span_sink().is_none());
        assert!(start_span(SpanKind::Request, "hypermind.request").is_none());
    }
}
