#![allow(clippy::missing_errors_doc)]

use crate::authority::{authority_for_event, cap_automatic_sensitivity};
use hm_core::{Error, ErrorCode};
use hm_schema::event::{EventKind, encode_event_envelope};
use hm_schema::events::{Authority, EventEnvelope, Retention};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum IngestSource {
    User,
    Assistant,
    ToolRuntime,
    Kernel,
    ExternalFeed,
    Derived,
    Document,
    Memory(String),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DoNotStoreReceipt {
    pub kind: EventKind,
    pub authority: Authority,
    pub content_hash: [u8; 32],
}

#[derive(Clone, Debug, PartialEq)]
pub enum IngestResult {
    Append(EventEnvelope),
    DoNotStore(DoNotStoreReceipt),
}

pub fn ingest(
    kind: EventKind,
    source: IngestSource,
    mut envelope: EventEnvelope,
    automatic_sensitivity: bool,
) -> Result<IngestResult, Error> {
    envelope.authority = match source {
        IngestSource::User if kind == EventKind::UserMsg => authority_for_event(kind),
        IngestSource::Assistant
            if matches!(kind, EventKind::DeliveredMsg | EventKind::Reasoning) =>
        {
            authority_for_event(kind)
        }
        IngestSource::ToolRuntime if kind == EventKind::ToolResult => authority_for_event(kind),
        IngestSource::Kernel if runtime_event(kind) => authority_for_event(kind),
        IngestSource::ExternalFeed
            if matches!(kind, EventKind::ProviderFrame | EventKind::MediaRef) =>
        {
            Authority::ExternalObserved
        }
        IngestSource::Derived
            if matches!(
                kind,
                EventKind::Assertion
                    | EventKind::Consolidation
                    | EventKind::Embedding
                    | EventKind::ProposedAssertion
                    | EventKind::MemoryMinted
                    | EventKind::MemoryRevised
                    | EventKind::MemoryMerged
                    | EventKind::EdgeAsserted
                    | EventKind::EdgeRetracted
            ) =>
        {
            authority_for_event(kind)
        }
        IngestSource::Document
            if matches!(
                kind,
                EventKind::DocumentIngested
                    | EventKind::DocumentExtracted
                    | EventKind::DocumentChunked
            ) =>
        {
            authority_for_event(kind)
        }
        IngestSource::Memory(namespace) => {
            if namespace.starts_with("memory.") {
                return Err(Error::new(ErrorCode::ForbiddenKind));
            }
            return Err(Error::new(ErrorCode::InvalidArgument));
        }
        _ => return Err(Error::new(ErrorCode::InvalidKind)),
    };
    if automatic_sensitivity {
        envelope.sensitivity = cap_automatic_sensitivity(envelope.sensitivity);
    }
    if envelope.retention == Retention::DoNotStore {
        let content_hash = *blake3::hash(&encode_event_envelope(&envelope)).as_bytes();
        return Ok(IngestResult::DoNotStore(DoNotStoreReceipt {
            kind,
            authority: envelope.authority,
            content_hash,
        }));
    }
    Ok(IngestResult::Append(envelope))
}

const fn runtime_event(kind: EventKind) -> bool {
    matches!(
        kind,
        EventKind::ToolCall
            | EventKind::Effect
            | EventKind::Approval
            | EventKind::Outcome
            | EventKind::Checkpoint
            | EventKind::Supervisor
            | EventKind::Recovery
            | EventKind::IntentSet
            | EventKind::LoopOpened
            | EventKind::LoopClosed
            | EventKind::Retract
            | EventKind::Attestation
            | EventKind::Binding
            | EventKind::MemoryFaded
            | EventKind::ConsolidationOpened
            | EventKind::ConsolidationPhase
            | EventKind::ConsolidationClosed
            | EventKind::ConsolidationRetracted
            | EventKind::Reviewed
    )
}
