#![allow(clippy::missing_errors_doc)]

use crate::events::{
    Assertion, AttentionDecided, AttestationDisposition, Authority, Binding, Consolidation,
    ConsolidationClosed, ConsolidationOpened, ConsolidationPhase, ConsolidationPhaseName,
    ConsolidationRetracted, DocumentChunked, DocumentExtracted, DocumentIngested, EdgeAsserted,
    EdgeRetracted, Effect, Embedding, EventEnvelope, EventEnvelopeRef, EventPayload,
    ExpectedPredicate, IntentionCancelled, IntentionFired, IntentionSet, LoopCloseReason,
    LoopClosed, MemoryFaded, MemoryMerged, MemoryMinted, MemoryRevised, Outcome, OutcomeObserved,
    Predicted, ProcedureAdopted, ProcedureImported, ProcedureImprovementProposed, ProcedureMined,
    ProcedureRevised, ProcedureSupport, ProposedAssertion, ProvenanceRange, Retract, Reviewed,
    SourceConnectorBound, SourceDeliveryAccepted, SourceDeliverySettled, SourceDeliveryState,
    SourceRevisionObserved, ToolResult, VocabularyImported, VocabularyTerm, WakeTrigger,
};
use hm_core::{Error, ErrorCode, LSN};
use planus::ReadAsRoot;
use std::fmt::Write as _;

use crate::validate::authority::{validate_observed_evidence, validate_optional_observed_evidence};

pub const CURRENT_SCHEMA_VERSION: u16 = 2;
pub const MAXIMUM_EVENT_BYTES: usize = 16 * 1024 * 1024;
pub const MAXIMUM_IDENTIFIER_BYTES: usize = 4096;
pub const MAXIMUM_PLAYBOOK_BYTES: usize = 65_536;
pub const MAXIMUM_VOCABULARY_TERMS: usize = 4096;
pub const MAXIMUM_VOCABULARY_ALIASES: usize = 32;
pub const MAXIMUM_VOCABULARY_NAME_BYTES: usize = 512;
pub const REPOSITORY_SNAPSHOT_PROVIDER: &str = "hypermind.repository-snapshot.v1";
pub const REPOSITORY_EXTRACT_MODEL_ID: &str = "repository-graph-extract";
pub const REPOSITORY_EXTRACT_PROMPT_ID: &str = "repository-graph-extract/v1";
pub const REPOSITORY_EXTRACT_PROMPT_VERSION: u16 = 1;
pub const REPOSITORY_EXTRACT_RUN_PREFIX: &str = "repository-graph/";

const MAXIMUM_FAILURE_EVIDENCE: usize = 256;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Boundary {
    Disk,
    Socket,
    Import,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum EventKind {
    UserMsg = 1,
    DeliveredMsg = 2,
    ToolCall = 3,
    ToolResult = 4,
    Reasoning = 5,
    ProviderFrame = 6,
    MediaRef = 7,
    Effect = 8,
    Approval = 9,
    Outcome = 10,
    Checkpoint = 11,
    Supervisor = 12,
    Recovery = 13,
    IntentSet = 14,
    LoopOpened = 15,
    LoopClosed = 16,
    Assertion = 17,
    Consolidation = 18,
    Embedding = 19,
    Retract = 20,
    Attestation = 21,
    Binding = 22,
    ProposedAssertion = 23,
    MemoryMinted = 24,
    MemoryRevised = 25,
    MemoryMerged = 26,
    MemoryFaded = 27,
    EdgeAsserted = 28,
    EdgeRetracted = 29,
    ConsolidationOpened = 30,
    ConsolidationPhase = 31,
    ConsolidationClosed = 32,
    ConsolidationRetracted = 33,
    Reviewed = 34,
    IntentionSet = 35,
    IntentionFired = 36,
    AttentionDecided = 37,
    IntentionCancelled = 38,
    Predicted = 39,
    OutcomeObserved = 40,
    ProcedureMined = 41,
    ProcedureRevised = 42,
    ProcedureAdopted = 43,
    VocabularyImported = 44,
    DocumentIngested = 45,
    DocumentExtracted = 46,
    DocumentChunked = 47,
    SourceConnectorBound = 48,
    SourceDeliveryAccepted = 49,
    SourceDeliverySettled = 50,
    SourceRevisionObserved = 51,
    ProcedureImported = 52,
    ProcedureImprovementProposed = 53,
}

impl EventKind {
    #[must_use]
    pub const fn is_wave_one(self) -> bool {
        matches!(
            self,
            Self::UserMsg
                | Self::DeliveredMsg
                | Self::ToolCall
                | Self::ToolResult
                | Self::Reasoning
                | Self::Attestation
        )
    }

    #[must_use]
    pub const fn is_wave_two(self) -> bool {
        self.is_wave_one()
            || matches!(
                self,
                Self::Effect
                    | Self::Approval
                    | Self::Outcome
                    | Self::Checkpoint
                    | Self::Supervisor
                    | Self::Recovery
                    | Self::IntentSet
                    | Self::LoopOpened
                    | Self::LoopClosed
                    | Self::Binding
            )
    }

    #[must_use]
    pub const fn is_wave_three(self) -> bool {
        self.is_wave_two() || matches!(self, Self::ProviderFrame | Self::MediaRef)
    }

    #[must_use]
    pub const fn is_wave_four(self) -> bool {
        self.is_wave_three() || matches!(self, Self::Embedding)
    }

    #[must_use]
    pub const fn is_wave_five(self) -> bool {
        self.is_wave_four()
            || matches!(
                self,
                Self::Assertion | Self::Consolidation | Self::Retract | Self::ProposedAssertion
            )
    }

    #[must_use]
    pub const fn is_wave_six(self) -> bool {
        self.is_wave_five()
            || matches!(
                self,
                Self::MemoryMinted
                    | Self::MemoryRevised
                    | Self::MemoryMerged
                    | Self::MemoryFaded
                    | Self::EdgeAsserted
                    | Self::EdgeRetracted
                    | Self::ConsolidationOpened
                    | Self::ConsolidationPhase
                    | Self::ConsolidationClosed
                    | Self::ConsolidationRetracted
                    | Self::Reviewed
            )
    }

    #[must_use]
    pub const fn is_wave_seven(self) -> bool {
        self.is_wave_six()
            || matches!(
                self,
                Self::IntentionSet
                    | Self::IntentionFired
                    | Self::AttentionDecided
                    | Self::IntentionCancelled
                    | Self::Predicted
                    | Self::OutcomeObserved
                    | Self::ProcedureMined
                    | Self::ProcedureRevised
                    | Self::ProcedureAdopted
                    | Self::VocabularyImported
                    | Self::DocumentIngested
                    | Self::DocumentExtracted
                    | Self::DocumentChunked
                    | Self::SourceConnectorBound
                    | Self::SourceDeliveryAccepted
                    | Self::SourceDeliverySettled
                    | Self::SourceRevisionObserved
                    | Self::ProcedureImported
                    | Self::ProcedureImprovementProposed
            )
    }

    #[must_use]
    pub const fn is_llm_derived(self) -> bool {
        matches!(
            self,
            Self::MemoryMinted
                | Self::MemoryRevised
                | Self::MemoryMerged
                | Self::EdgeAsserted
                | Self::EdgeRetracted
        )
    }

    #[must_use]
    pub const fn requires_run_id(self) -> bool {
        matches!(
            self,
            Self::MemoryMinted
                | Self::MemoryRevised
                | Self::MemoryMerged
                | Self::MemoryFaded
                | Self::EdgeAsserted
                | Self::EdgeRetracted
                | Self::ConsolidationOpened
                | Self::ConsolidationPhase
                | Self::ConsolidationClosed
                | Self::ConsolidationRetracted
                | Self::Reviewed
                | Self::DocumentExtracted
                | Self::DocumentChunked
        )
    }
}

impl TryFrom<u8> for EventKind {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::UserMsg),
            2 => Ok(Self::DeliveredMsg),
            3 => Ok(Self::ToolCall),
            4 => Ok(Self::ToolResult),
            5 => Ok(Self::Reasoning),
            6 => Ok(Self::ProviderFrame),
            7 => Ok(Self::MediaRef),
            8 => Ok(Self::Effect),
            9 => Ok(Self::Approval),
            10 => Ok(Self::Outcome),
            11 => Ok(Self::Checkpoint),
            12 => Ok(Self::Supervisor),
            13 => Ok(Self::Recovery),
            14 => Ok(Self::IntentSet),
            15 => Ok(Self::LoopOpened),
            16 => Ok(Self::LoopClosed),
            17 => Ok(Self::Assertion),
            18 => Ok(Self::Consolidation),
            19 => Ok(Self::Embedding),
            20 => Ok(Self::Retract),
            21 => Ok(Self::Attestation),
            22 => Ok(Self::Binding),
            23 => Ok(Self::ProposedAssertion),
            24 => Ok(Self::MemoryMinted),
            25 => Ok(Self::MemoryRevised),
            26 => Ok(Self::MemoryMerged),
            27 => Ok(Self::MemoryFaded),
            28 => Ok(Self::EdgeAsserted),
            29 => Ok(Self::EdgeRetracted),
            30 => Ok(Self::ConsolidationOpened),
            31 => Ok(Self::ConsolidationPhase),
            32 => Ok(Self::ConsolidationClosed),
            33 => Ok(Self::ConsolidationRetracted),
            34 => Ok(Self::Reviewed),
            35 => Ok(Self::IntentionSet),
            36 => Ok(Self::IntentionFired),
            37 => Ok(Self::AttentionDecided),
            38 => Ok(Self::IntentionCancelled),
            39 => Ok(Self::Predicted),
            40 => Ok(Self::OutcomeObserved),
            41 => Ok(Self::ProcedureMined),
            42 => Ok(Self::ProcedureRevised),
            43 => Ok(Self::ProcedureAdopted),
            44 => Ok(Self::VocabularyImported),
            45 => Ok(Self::DocumentIngested),
            46 => Ok(Self::DocumentExtracted),
            47 => Ok(Self::DocumentChunked),
            48 => Ok(Self::SourceConnectorBound),
            49 => Ok(Self::SourceDeliveryAccepted),
            50 => Ok(Self::SourceDeliverySettled),
            51 => Ok(Self::SourceRevisionObserved),
            52 => Ok(Self::ProcedureImported),
            53 => Ok(Self::ProcedureImprovementProposed),
            _ => Err(()),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HistorySource {
    LedgerEvent,
    Memory,
    Summary,
    Reconstruction,
}

pub trait EventHistory {
    fn kind_at(&self, lsn: LSN) -> Option<EventKind>;

    fn authority_at(&self, _lsn: LSN) -> Option<Authority> {
        None
    }

    fn source_at(&self, _lsn: LSN) -> HistorySource {
        HistorySource::LedgerEvent
    }
}

impl<F> EventHistory for F
where
    F: Fn(LSN) -> Option<EventKind>,
{
    fn kind_at(&self, lsn: LSN) -> Option<EventKind> {
        self(lsn)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct VerifiedEvent {
    pub envelope: EventEnvelope,
    pub kind: EventKind,
    pub boundary: Boundary,
}

#[must_use]
pub fn encode_event_envelope(envelope: &EventEnvelope) -> Vec<u8> {
    let mut builder = planus::Builder::new();
    let encoded = builder.finish(envelope, None);
    finish_with_identifier(encoded, *b"NCEV")
}

pub fn verify_event(
    encoded: &[u8],
    expected_kind: EventKind,
    boundary: Boundary,
) -> Result<VerifiedEvent, Error> {
    verify_event_with_history(encoded, expected_kind, boundary, &|_| None)
}

pub fn verify_event_with_history(
    encoded: &[u8],
    expected_kind: EventKind,
    boundary: Boundary,
    history: &impl EventHistory,
) -> Result<VerifiedEvent, Error> {
    if encoded.is_empty() || encoded.len() > MAXIMUM_EVENT_BYTES {
        return Err(Error::new(ErrorCode::SchemaInvalid));
    }
    require_identifier(encoded, *b"NCEV", ErrorCode::SchemaInvalid)?;
    let envelope_ref = EventEnvelopeRef::read_as_root(encoded)
        .map_err(|_| Error::new(ErrorCode::SchemaInvalid))?;
    let envelope =
        EventEnvelope::try_from(envelope_ref).map_err(|_| Error::new(ErrorCode::SchemaInvalid))?;
    if envelope.schema_version == 0 || envelope.schema_version > CURRENT_SCHEMA_VERSION {
        return Err(Error::new(ErrorCode::SchemaVersion));
    }
    if !expected_kind.is_wave_seven() || payload_kind(&envelope.payload) != expected_kind {
        return Err(Error::new(ErrorCode::ForbiddenKind));
    }
    validate_envelope(&envelope, expected_kind)?;
    let legacy_evidence_allowed = envelope.schema_version == 1 && boundary != Boundary::Socket;
    validate_payload(&envelope.payload, history, legacy_evidence_allowed)?;
    Ok(VerifiedEvent {
        envelope,
        kind: expected_kind,
        boundary,
    })
}

fn require_identifier(encoded: &[u8], identifier: [u8; 4], code: ErrorCode) -> Result<(), Error> {
    if encoded.get(4..8) == Some(identifier.as_slice()) {
        Ok(())
    } else {
        Err(Error::new(code))
    }
}

fn finish_with_identifier(encoded: &[u8], identifier: [u8; 4]) -> Vec<u8> {
    let root_offset = u32::from_le_bytes(encoded[..4].try_into().expect("Planus root offset"));
    let mut output = Vec::with_capacity(encoded.len() + identifier.len());
    output.extend_from_slice(&(root_offset + 4).to_le_bytes());
    output.extend_from_slice(&identifier);
    output.extend_from_slice(&encoded[4..]);
    output
}

fn validate_envelope(envelope: &EventEnvelope, kind: EventKind) -> Result<(), Error> {
    if envelope
        .connection_id
        .as_ref()
        .is_some_and(|value| value.len() != 16)
        || envelope
            .run_id
            .as_ref()
            .is_some_and(|value| value.is_empty() || value.len() > MAXIMUM_IDENTIFIER_BYTES)
        || (envelope.client_event_count == 0 && envelope.client_event_index != 0)
        || (envelope.client_event_count != 0
            && envelope.client_event_index >= envelope.client_event_count)
    {
        return Err(Error::new(ErrorCode::SchemaInvalid));
    }
    if let Some(model) = &envelope.model_provenance
        && (model.model_id.is_empty()
            || model.prompt_id.is_empty()
            || (kind.is_llm_derived()
                && (model.prompt_version == 0
                    || !model.call_id.as_deref().is_some_and(bounded_identifier)
                    || (model.input_tokens.saturating_add(model.output_tokens) == 0
                        && !is_cortex_import(envelope, kind)
                        && !is_repository_extraction(envelope, kind)))))
    {
        return Err(Error::new(ErrorCode::SchemaInvalid));
    }
    if kind.requires_run_id() && !envelope.run_id.as_deref().is_some_and(bounded_identifier) {
        return Err(Error::new(ErrorCode::SchemaInvalid));
    }
    if kind.is_llm_derived() && envelope.model_provenance.is_none() {
        return Err(Error::new(ErrorCode::SchemaInvalid));
    }
    if kind == EventKind::ProcedureAdopted && envelope.authority != Authority::UserAsserted {
        return Err(Error::new(ErrorCode::ProtectedTypeWrite));
    }
    if kind == EventKind::VocabularyImported && envelope.authority != Authority::UserAsserted {
        return Err(Error::new(ErrorCode::ProtectedTypeWrite));
    }
    if kind == EventKind::SourceConnectorBound && envelope.authority != Authority::UserAsserted {
        return Err(Error::new(ErrorCode::ProtectedTypeWrite));
    }
    if matches!(
        kind,
        EventKind::SourceDeliveryAccepted | EventKind::SourceRevisionObserved
    ) && envelope.authority != Authority::ExternalObserved
    {
        return Err(Error::new(ErrorCode::ProtectedTypeWrite));
    }
    if kind == EventKind::SourceDeliverySettled && envelope.authority != Authority::RuntimeFact {
        return Err(Error::new(ErrorCode::ProtectedTypeWrite));
    }
    if kind == EventKind::ProcedureImported && envelope.authority != Authority::ExternalObserved {
        return Err(Error::new(ErrorCode::ProtectedTypeWrite));
    }
    if kind == EventKind::ProcedureImprovementProposed
        && envelope.authority != Authority::DerivedInference
    {
        return Err(Error::new(ErrorCode::ProtectedTypeWrite));
    }
    Ok(())
}

fn is_repository_extraction(envelope: &EventEnvelope, kind: EventKind) -> bool {
    let Some(model) = envelope.model_provenance.as_ref() else {
        return false;
    };
    let Some(digest) = model.call_id.as_deref().filter(|digest| digest.len() == 32) else {
        return false;
    };
    let mut expected_run = String::from(REPOSITORY_EXTRACT_RUN_PREFIX);
    expected_run.reserve(digest.len() * 2);
    for byte in digest {
        write!(expected_run, "{byte:02x}").expect("writing to a String cannot fail");
    }
    matches!(
        kind,
        EventKind::MemoryMinted
            | EventKind::MemoryRevised
            | EventKind::EdgeAsserted
            | EventKind::EdgeRetracted
    ) && envelope.authority == Authority::DerivedInference
        && envelope.run_id.as_deref() == Some(expected_run.as_bytes())
        && model.model_id == REPOSITORY_EXTRACT_MODEL_ID
        && model.prompt_id == REPOSITORY_EXTRACT_PROMPT_ID
        && model.prompt_version == REPOSITORY_EXTRACT_PROMPT_VERSION
        && model.temperature == 0.0
        && model.input_tokens == 0
        && model.output_tokens == 0
        && model.cache_read_tokens == 0
        && model.cache_write_tokens == 0
        && model.cost_microusd == 0
}

fn is_cortex_import(envelope: &EventEnvelope, kind: EventKind) -> bool {
    let Some(model) = envelope.model_provenance.as_ref() else {
        return false;
    };
    let Some(digest) = model.call_id.as_deref().filter(|digest| digest.len() == 32) else {
        return false;
    };
    let mut expected_run = String::from("cortex-store-import/");
    expected_run.reserve(digest.len() * 2);
    for byte in digest {
        write!(expected_run, "{byte:02x}").expect("writing to a String cannot fail");
    }
    matches!(kind, EventKind::MemoryMinted | EventKind::EdgeAsserted)
        && envelope.authority == Authority::DerivedInference
        && envelope.run_id.as_deref() == Some(expected_run.as_bytes())
        && model.model_id == "cortex-store-import"
        && model.prompt_id == "cortex-store-import/v1"
        && model.prompt_version == 1
        && model.temperature == 0.0
        && model.input_tokens == 0
        && model.output_tokens == 0
        && model.cache_read_tokens == 0
        && model.cache_write_tokens == 0
        && model.cost_microusd == 0
}

#[allow(clippy::too_many_lines)]
fn validate_payload(
    payload: &EventPayload,
    history: &impl EventHistory,
    legacy_evidence_allowed: bool,
) -> Result<(), Error> {
    match payload {
        EventPayload::UserMsg(_) | EventPayload::DeliveredMsg(_) | EventPayload::Reasoning(_) => {
            Ok(())
        }
        EventPayload::ProviderFrame(_) | EventPayload::MediaRef(_) => {
            validate_external_payload(payload)
        }
        EventPayload::ToolCall(value) => {
            if bounded_identifier(&value.call_id) && !value.tool_name.is_empty() {
                Ok(())
            } else {
                Err(Error::new(ErrorCode::SchemaInvalid))
            }
        }
        EventPayload::ToolResult(value) => validate_tool_result(value, history),
        EventPayload::Effect(value) => validate_effect(value, history),
        EventPayload::Approval(value) => {
            if bounded_identifier(&value.effect_id) {
                Ok(())
            } else {
                Err(Error::new(ErrorCode::SchemaInvalid))
            }
        }
        EventPayload::Outcome(value) => validate_outcome(value, history, legacy_evidence_allowed),
        EventPayload::Checkpoint(value) => {
            if value.cursor.is_empty() {
                Err(Error::new(ErrorCode::SchemaInvalid))
            } else {
                Ok(())
            }
        }
        EventPayload::Supervisor(value) => {
            if value.code.is_empty() {
                Err(Error::new(ErrorCode::SchemaInvalid))
            } else {
                Ok(())
            }
        }
        EventPayload::Recovery(value) => {
            if value.code.is_empty() || value.target_lsn == 0 {
                Err(Error::new(ErrorCode::SchemaInvalid))
            } else {
                Ok(())
            }
        }
        EventPayload::IntentSet(value) => {
            if value.objective.is_empty() {
                Err(Error::new(ErrorCode::SchemaInvalid))
            } else {
                Ok(())
            }
        }
        EventPayload::LoopOpened(value) => {
            if bounded_identifier(&value.loop_id) && !value.objective.is_empty() {
                Ok(())
            } else {
                Err(Error::new(ErrorCode::SchemaInvalid))
            }
        }
        EventPayload::LoopClosed(value) => {
            validate_loop_closed(value, history, legacy_evidence_allowed)
        }
        EventPayload::Attestation(value) => {
            if value.target_lsn != 0
                && matches!(
                    value.disposition,
                    AttestationDisposition::Used
                        | AttestationDisposition::Ignored
                        | AttestationDisposition::Helpful
                        | AttestationDisposition::Harmful
                )
            {
                Ok(())
            } else {
                Err(Error::new(ErrorCode::SchemaInvalid))
            }
        }
        EventPayload::Binding(value) => validate_binding(value),
        EventPayload::Embedding(value) => validate_embedding(value),
        EventPayload::Assertion(value) => validate_assertion(value),
        EventPayload::ProposedAssertion(value) => validate_proposed_assertion(value),
        EventPayload::Consolidation(value) => validate_consolidation(value),
        EventPayload::Retract(value) => validate_retract(value),
        EventPayload::MemoryMinted(_)
        | EventPayload::MemoryRevised(_)
        | EventPayload::MemoryMerged(_)
        | EventPayload::MemoryFaded(_)
        | EventPayload::EdgeAsserted(_)
        | EventPayload::EdgeRetracted(_)
        | EventPayload::ConsolidationOpened(_)
        | EventPayload::ConsolidationPhase(_)
        | EventPayload::ConsolidationClosed(_)
        | EventPayload::ConsolidationRetracted(_)
        | EventPayload::Reviewed(_) => validate_wave_six_payload(payload),
        EventPayload::IntentionSet(value) => validate_intention_set(value),
        EventPayload::IntentionFired(value) => validate_intention_fired(value),
        EventPayload::AttentionDecided(value) => validate_attention_decided(value),
        EventPayload::IntentionCancelled(value) => validate_intention_cancelled(value),
        EventPayload::Predicted(value) => validate_predicted(value),
        EventPayload::OutcomeObserved(value) => validate_outcome_observed(value, history),
        EventPayload::ProcedureMined(value) => validate_procedure_mined(value),
        EventPayload::ProcedureRevised(value) => validate_procedure_revised(value),
        EventPayload::ProcedureAdopted(value) => validate_procedure_adopted(value),
        EventPayload::VocabularyImported(value) => validate_vocabulary_imported(value),
        EventPayload::DocumentIngested(value) => validate_document_ingested(value),
        EventPayload::DocumentExtracted(value) => validate_document_extracted(value),
        EventPayload::DocumentChunked(value) => validate_document_chunked(value),
        EventPayload::SourceConnectorBound(value) => validate_source_connector_bound(value),
        EventPayload::SourceDeliveryAccepted(value) => validate_source_delivery_accepted(value),
        EventPayload::SourceDeliverySettled(value) => validate_source_delivery_settled(value),
        EventPayload::SourceRevisionObserved(value) => validate_source_revision_observed(value),
        EventPayload::ProcedureImported(value) => validate_procedure_imported(value),
        EventPayload::ProcedureImprovementProposed(value) => {
            validate_procedure_improvement_proposed(value, history)
        }
    }
}

fn validate_source_connector_bound(value: &SourceConnectorBound) -> Result<(), Error> {
    if value.connector_id.len() != 16
        || value.consent_nonce.len() != 16
        || value.provider.is_empty()
        || value.external_account.is_empty()
        || value.credential_version == 0
    {
        return Err(Error::new(ErrorCode::SchemaInvalid));
    }
    Ok(())
}

fn validate_source_delivery_accepted(value: &SourceDeliveryAccepted) -> Result<(), Error> {
    if value.connector_id.len() != 16
        || !bounded_identifier(&value.delivery_id)
        || value.credential_version == 0
        || value.signed_at_ns == 0
        || value.body_digest.len() != 32
        || value.event_name.is_empty()
    {
        return Err(Error::new(ErrorCode::SchemaInvalid));
    }
    Ok(())
}

fn validate_source_delivery_settled(value: &SourceDeliverySettled) -> Result<(), Error> {
    if value.connector_id.len() != 16
        || !bounded_identifier(&value.delivery_id)
        || value.state == SourceDeliveryState::Accepted
        || value.detail.is_empty()
    {
        return Err(Error::new(ErrorCode::SchemaInvalid));
    }
    Ok(())
}

fn validate_source_revision_observed(value: &SourceRevisionObserved) -> Result<(), Error> {
    if value.connector_id.len() != 16
        || value.source_id.is_empty()
        || !bounded_identifier(&value.revision)
        || value.content_digest.len() != 32
        || value.observed_at_ns == 0
    {
        return Err(Error::new(ErrorCode::SchemaInvalid));
    }
    Ok(())
}

fn validate_intention_set(value: &IntentionSet) -> Result<(), Error> {
    if !bounded_identifier(&value.intention_id)
        || value.objective.is_empty()
        || value
            .trigger
            .as_ref()
            .is_none_or(|trigger| !valid_trigger(trigger))
        || value.expires_at_ns == 0
        || value.reply_route.is_empty()
    {
        Err(Error::new(ErrorCode::SchemaInvalid))
    } else {
        Ok(())
    }
}

fn valid_trigger(trigger: &WakeTrigger) -> bool {
    match trigger {
        WakeTrigger::WakeAt(value) => value.at_ns != 0,
        WakeTrigger::WakeSchedule(value) => !value.schedule.is_empty(),
        WakeTrigger::WakeChildTerminal(value) => bounded_identifier(&value.child_id),
        WakeTrigger::WakeProcessExit(value) => bounded_identifier(&value.process_id),
        WakeTrigger::WakeFileChanged(value) => !value.path.is_empty(),
        WakeTrigger::WakeRepositoryChanged(value) => !value.repository.is_empty(),
        WakeTrigger::WakeChannelMessage(value) => !value.channel.is_empty(),
        WakeTrigger::WakeExternalCondition(value) => !value.condition.is_empty(),
        WakeTrigger::WakeUserResponse(value) => bounded_identifier(&value.reply_to),
        WakeTrigger::WakeEntityMentioned(value) => bounded_identifier(&value.entity_id),
        WakeTrigger::WakeLoopClosed(value) => bounded_identifier(&value.loop_id),
        WakeTrigger::WakePredictionResolved(value) => bounded_identifier(&value.prediction_id),
        WakeTrigger::WakeBeliefChanged(value) => !value.canonical_identity.is_empty(),
    }
}

fn validate_intention_fired(value: &IntentionFired) -> Result<(), Error> {
    if bounded_identifier(&value.intention_id)
        && bounded_identifier(&value.wake_id)
        && value.trigger_lsn != 0
    {
        Ok(())
    } else {
        Err(Error::new(ErrorCode::SchemaInvalid))
    }
}

fn validate_attention_decided(value: &AttentionDecided) -> Result<(), Error> {
    if bounded_identifier(&value.intention_id)
        && bounded_identifier(&value.wake_id)
        && !value.reason.is_empty()
    {
        Ok(())
    } else {
        Err(Error::new(ErrorCode::SchemaInvalid))
    }
}

fn validate_intention_cancelled(value: &IntentionCancelled) -> Result<(), Error> {
    if bounded_identifier(&value.intention_id) && !value.reason.is_empty() {
        Ok(())
    } else {
        Err(Error::new(ErrorCode::SchemaInvalid))
    }
}

fn validate_predicted(value: &Predicted) -> Result<(), Error> {
    if !bounded_identifier(&value.prediction_id)
        || value.revision == 0
        || value
            .task_id
            .as_deref()
            .is_some_and(|identifier| !bounded_identifier(identifier))
        || value
            .attempt_id
            .as_deref()
            .is_some_and(|identifier| !bounded_identifier(identifier))
        || value
            .operation_id
            .as_deref()
            .is_some_and(|identifier| !bounded_identifier(identifier))
        || value.mechanism.is_empty()
        || value.predicates.is_empty()
        || value.predicates.len() > 16
        || value
            .predicates
            .iter()
            .any(|predicate| !valid_predicate(predicate))
        || value.deadline_ns == 0
        || value.uncertainty.is_empty()
    {
        Err(Error::new(ErrorCode::SchemaInvalid))
    } else {
        Ok(())
    }
}

fn valid_predicate(predicate: &ExpectedPredicate) -> bool {
    !predicate.scope.is_empty()
        && predicate
            .property
            .as_ref()
            .is_none_or(|property| !property.is_empty())
        && predicate
            .expected
            .as_ref()
            .is_none_or(|expected| !expected.is_empty())
}

fn validate_outcome_observed(
    value: &OutcomeObserved,
    history: &impl EventHistory,
) -> Result<(), Error> {
    if !bounded_identifier(&value.prediction_id)
        || value.revision == 0
        || value.evaluator_version.is_empty()
    {
        return Err(Error::new(ErrorCode::SchemaInvalid));
    }
    validate_observed_evidence(&value.observation_lsns, history)
}

fn validate_procedure_mined(value: &ProcedureMined) -> Result<(), Error> {
    validate_procedure(
        &value.procedure_id,
        &value.strategy,
        &value.expected_outcomes,
        &value.preconditions,
        &value.supports,
        value.failures.as_deref(),
        value.counterexamples.as_deref(),
    )
}

fn validate_procedure_revised(value: &ProcedureRevised) -> Result<(), Error> {
    if value.previous_lsn == 0 {
        return Err(Error::new(ErrorCode::SchemaInvalid));
    }
    validate_procedure(
        &value.procedure_id,
        &value.strategy,
        &value.expected_outcomes,
        &value.preconditions,
        &value.supports,
        value.failures.as_deref(),
        value.counterexamples.as_deref(),
    )
}

fn validate_procedure(
    procedure_id: &[u8],
    strategy: &str,
    expected_outcomes: &[String],
    preconditions: &[String],
    supports: &[ProcedureSupport],
    failures: Option<&[u64]>,
    counterexamples: Option<&[u64]>,
) -> Result<(), Error> {
    if !bounded_identifier(procedure_id)
        || strategy.is_empty()
        || expected_outcomes.is_empty()
        || expected_outcomes.iter().any(String::is_empty)
        || preconditions.iter().any(String::is_empty)
        || supports.is_empty()
        || supports.iter().any(|support| {
            !bounded_identifier(&support.source_root)
                || support.conversation.len() != 16
                || support.episode_lsn == 0
        })
        || failures.is_some_and(|lsns| lsns.contains(&0))
        || counterexamples.is_some_and(|lsns| lsns.contains(&0))
    {
        Err(Error::new(ErrorCode::SchemaInvalid))
    } else {
        Ok(())
    }
}

fn validate_procedure_adopted(value: &ProcedureAdopted) -> Result<(), Error> {
    if bounded_identifier(&value.procedure_id) && value.procedure_lsn != 0 {
        Ok(())
    } else {
        Err(Error::new(ErrorCode::SchemaInvalid))
    }
}

fn validate_procedure_imported(value: &ProcedureImported) -> Result<(), Error> {
    if !bounded_identifier(&value.procedure_id)
        || !bounded_identifier(value.name.as_bytes())
        || value.strategy.is_empty()
        || value.expected_outcomes.is_empty()
        || value.expected_outcomes.iter().any(String::is_empty)
        || value.preconditions.iter().any(String::is_empty)
        || value.declared_tools.iter().any(String::is_empty)
        || value.instructions.is_empty()
        || value.instructions.len() > MAXIMUM_PLAYBOOK_BYTES
        || !bounded_identifier(value.source_uri.as_bytes())
        || value.source_digest.len() != 32
    {
        Err(Error::new(ErrorCode::SchemaInvalid))
    } else {
        Ok(())
    }
}

fn validate_procedure_improvement_proposed(
    value: &ProcedureImprovementProposed,
    history: &impl EventHistory,
) -> Result<(), Error> {
    if !bounded_identifier(&value.proposal_id)
        || !bounded_identifier(&value.procedure_id)
        || value.base_lsn == 0
        || value.strategy.is_empty()
        || value.expected_outcomes.is_empty()
        || value.expected_outcomes.iter().any(String::is_empty)
        || value.preconditions.iter().any(String::is_empty)
        || value.rationale.is_empty()
        || value.rationale.len() > MAXIMUM_PLAYBOOK_BYTES
        || value.failure_lsns.is_empty()
        || value.failure_lsns.len() > MAXIMUM_FAILURE_EVIDENCE
        || value.failure_lsns.contains(&0)
        || value.failure_lsns.windows(2).any(|pair| pair[0] >= pair[1])
    {
        return Err(Error::new(ErrorCode::SchemaInvalid));
    }
    validate_observed_evidence(&value.failure_lsns, history)
}

fn validate_vocabulary_imported(value: &VocabularyImported) -> Result<(), Error> {
    if !bounded_identifier(&value.vocabulary_id)
        || value.version == 0
        || value.source_uri.is_empty()
        || value.source_media_type.is_empty()
        || value.source_digest.len() != 32
        || value.terms.is_empty()
        || value.terms.len() > MAXIMUM_VOCABULARY_TERMS
        || !value.terms.iter().all(valid_vocabulary_term)
        || value
            .terms
            .windows(2)
            .any(|pair| pair[0].term_id >= pair[1].term_id)
    {
        Err(Error::new(ErrorCode::SchemaInvalid))
    } else {
        Ok(())
    }
}

fn valid_vocabulary_term(term: &VocabularyTerm) -> bool {
    bounded_vocabulary_name(&term.term_id)
        && bounded_vocabulary_name(&term.canonical_name)
        && term
            .parent_term_id
            .as_deref()
            .is_none_or(bounded_vocabulary_name)
        && term.aliases.as_deref().is_none_or(|aliases| {
            aliases.len() <= MAXIMUM_VOCABULARY_ALIASES
                && aliases
                    .iter()
                    .all(|alias| bounded_vocabulary_name(alias.as_str()))
        })
}

fn bounded_vocabulary_name(value: &str) -> bool {
    !value.is_empty() && value.len() <= MAXIMUM_VOCABULARY_NAME_BYTES
}

fn validate_document_ingested(value: &DocumentIngested) -> Result<(), Error> {
    if value.document_id.len() == 32
        && bounded_identifier(value.name.as_bytes())
        && bounded_identifier(value.media_type.as_bytes())
        && !value.content.is_empty()
        && value.content_digest.len() == 32
    {
        Ok(())
    } else {
        Err(Error::new(ErrorCode::SchemaInvalid))
    }
}

fn validate_document_extracted(value: &DocumentExtracted) -> Result<(), Error> {
    let reason = value
        .partial_reason
        .as_deref()
        .is_some_and(|reason| !reason.is_empty());
    let failed = value
        .failed_units
        .as_deref()
        .is_some_and(|units| !units.is_empty());
    if value.document_id.len() != 32
        || value.source_lsn == 0
        || !bounded_identifier(value.loader_id.as_bytes())
        || value.extraction_version == 0
        || value.text.is_empty()
        || value.text_digest.len() != 32
        || reason != failed
    {
        return Err(Error::new(ErrorCode::SchemaInvalid));
    }
    Ok(())
}

fn validate_document_chunked(value: &DocumentChunked) -> Result<(), Error> {
    if value.document_id.len() != 32
        || value.source_lsn == 0
        || value.extraction_version == 0
        || !bounded_identifier(value.chunker_id.as_bytes())
        || value.token_budget == 0
        || value.chunks.is_empty()
    {
        return Err(Error::new(ErrorCode::SchemaInvalid));
    }
    let mut expected_start = 0_u32;
    for (index, chunk) in value.chunks.iter().enumerate() {
        let ordinal = u32::try_from(index).map_err(|_| Error::new(ErrorCode::SchemaInvalid))?;
        if chunk.chunk_id.len() != 32
            || chunk.content_hash.len() != 32
            || chunk.ordinal != ordinal
            || chunk.byte_start != expected_start
            || chunk.byte_end <= chunk.byte_start
        {
            return Err(Error::new(ErrorCode::SchemaInvalid));
        }
        expected_start = chunk.byte_end;
    }
    Ok(())
}

fn validate_external_payload(payload: &EventPayload) -> Result<(), Error> {
    let valid = match payload {
        EventPayload::ProviderFrame(value) => {
            !value.provider.is_empty() && !value.api_content.is_empty()
        }
        EventPayload::MediaRef(value) => {
            !value.uri.is_empty() && !value.media_type.is_empty() && !value.digest.is_empty()
        }
        _ => false,
    };
    if valid {
        Ok(())
    } else {
        Err(Error::new(ErrorCode::SchemaInvalid))
    }
}

fn validate_wave_six_payload(payload: &EventPayload) -> Result<(), Error> {
    match payload {
        EventPayload::MemoryMinted(value) => validate_memory_minted(value),
        EventPayload::MemoryRevised(value) => validate_memory_revised(value),
        EventPayload::MemoryMerged(value) => validate_memory_merged(value),
        EventPayload::MemoryFaded(value) => validate_memory_faded(value),
        EventPayload::EdgeAsserted(value) => validate_edge_asserted(value),
        EventPayload::EdgeRetracted(value) => validate_edge_retracted(value),
        EventPayload::ConsolidationOpened(value) => validate_consolidation_opened(value),
        EventPayload::ConsolidationPhase(value) => validate_consolidation_phase(value),
        EventPayload::ConsolidationClosed(value) => validate_consolidation_closed(value),
        EventPayload::ConsolidationRetracted(value) => validate_consolidation_retracted(value),
        EventPayload::Reviewed(value) => validate_reviewed(value),
        _ => Err(Error::new(ErrorCode::ForbiddenKind)),
    }
}

fn validate_tool_result(value: &ToolResult, history: &impl EventHistory) -> Result<(), Error> {
    if !bounded_identifier(&value.call_id) || value.tool_call_lsn == 0 {
        return Err(Error::new(ErrorCode::SchemaInvalid));
    }
    require_prior_tool_call(value.tool_call_lsn, history)
}

fn validate_effect(value: &Effect, history: &impl EventHistory) -> Result<(), Error> {
    if !bounded_identifier(&value.effect_id) || value.tool_call_lsn == 0 {
        return Err(Error::new(ErrorCode::SchemaInvalid));
    }
    require_prior_tool_call(value.tool_call_lsn, history)
}

fn validate_outcome(
    value: &Outcome,
    history: &impl EventHistory,
    legacy_evidence_allowed: bool,
) -> Result<(), Error> {
    if !bounded_identifier(&value.effect_id) {
        return Err(Error::new(ErrorCode::SchemaInvalid));
    }
    validate_optional_observed_evidence(
        value.evidence_lsns.as_deref(),
        history,
        legacy_evidence_allowed,
    )
}

fn validate_loop_closed(
    value: &LoopClosed,
    history: &impl EventHistory,
    legacy_evidence_allowed: bool,
) -> Result<(), Error> {
    if !bounded_identifier(&value.loop_id) {
        return Err(Error::new(ErrorCode::SchemaInvalid));
    }
    if value.reason == LoopCloseReason::Done {
        validate_optional_observed_evidence(
            value.evidence_lsns.as_deref(),
            history,
            legacy_evidence_allowed,
        )?;
    }
    Ok(())
}

fn validate_binding(value: &Binding) -> Result<(), Error> {
    let task = value.task.as_deref().is_some_and(bounded_identifier);
    let scope = value.scope.as_deref().is_some_and(bounded_identifier);
    if task == scope
        || value.canonical_entity.is_empty()
        || value.property.is_empty()
        || value.evidence_lsn == 0
        || value.revision.is_empty()
        || value.revision.len() > MAXIMUM_IDENTIFIER_BYTES
        || value.freshness_requirement_ns == 0
    {
        Err(Error::new(ErrorCode::SchemaInvalid))
    } else {
        Ok(())
    }
}

fn validate_embedding(value: &Embedding) -> Result<(), Error> {
    let dimension =
        usize::try_from(value.dimension).map_err(|_| Error::new(ErrorCode::SchemaInvalid))?;
    if value.target_lsn == 0
        || value.space_id.is_empty()
        || value.space_id.len() > MAXIMUM_IDENTIFIER_BYTES
        || dimension == 0
        || value.quantized.len() != dimension
        || value.binary_prefilter.len() != dimension.div_ceil(8)
    {
        Err(Error::new(ErrorCode::SchemaInvalid))
    } else {
        Ok(())
    }
}

fn validate_assertion(value: &Assertion) -> Result<(), Error> {
    validate_belief_fields(
        &value.belief_id,
        &value.canonical_identity,
        &value.value,
        value.valid_from_ns,
        value.valid_to_ns,
        &value.provenance,
        value.conflict_domain.as_deref(),
    )
}

fn validate_proposed_assertion(value: &ProposedAssertion) -> Result<(), Error> {
    validate_belief_fields(
        &value.belief_id,
        &value.canonical_identity,
        &value.value,
        value.valid_from_ns,
        value.valid_to_ns,
        &value.provenance,
        value.conflict_domain.as_deref(),
    )
}

fn validate_consolidation(value: &Consolidation) -> Result<(), Error> {
    if value.assertions.is_empty() {
        return Err(Error::new(ErrorCode::SchemaInvalid));
    }
    value.assertions.iter().try_for_each(validate_assertion)
}

fn validate_retract(value: &Retract) -> Result<(), Error> {
    if !bounded_identifier(&value.belief_id) {
        return Err(Error::new(ErrorCode::SchemaInvalid));
    }
    validate_provenance(&value.provenance)
}

fn validate_memory_minted(value: &MemoryMinted) -> Result<(), Error> {
    validate_memory_fields(
        &value.memory_id,
        &value.name,
        &value.definition,
        &value.tags,
        value.salience_micros,
        &value.citations,
    )
}

fn validate_memory_revised(value: &MemoryRevised) -> Result<(), Error> {
    if value.previous_lsn == 0 {
        return Err(Error::new(ErrorCode::SchemaInvalid));
    }
    validate_memory_fields(
        &value.memory_id,
        &value.name,
        &value.definition,
        &value.tags,
        value.salience_micros,
        &value.citations,
    )
}

fn validate_memory_merged(value: &MemoryMerged) -> Result<(), Error> {
    if value.merged_memory_ids.len() < 2
        || value
            .merged_memory_ids
            .iter()
            .any(|source| !bounded_identifier(&source.value) || source.value == value.memory_id)
        || value
            .merged_memory_ids
            .iter()
            .enumerate()
            .any(|(index, source)| {
                value.merged_memory_ids[index + 1..]
                    .iter()
                    .any(|other| source.value == other.value)
            })
    {
        return Err(Error::new(ErrorCode::SchemaInvalid));
    }
    validate_memory_fields(
        &value.memory_id,
        &value.name,
        &value.definition,
        &value.tags,
        value.salience_micros,
        &value.citations,
    )
}

fn validate_memory_fields(
    memory_id: &[u8],
    name: &str,
    definition: &[u8],
    tags: &[String],
    salience_micros: u32,
    citations: &[ProvenanceRange],
) -> Result<(), Error> {
    if !bounded_identifier(memory_id)
        || name.is_empty()
        || name.len() > MAXIMUM_IDENTIFIER_BYTES
        || definition.is_empty()
        || definition.len() > MAXIMUM_EVENT_BYTES
        || tags.is_empty()
        || tags
            .iter()
            .any(|tag| tag.is_empty() || tag.len() > MAXIMUM_IDENTIFIER_BYTES)
        || salience_micros > 1_000_000
    {
        return Err(Error::new(ErrorCode::SchemaInvalid));
    }
    validate_provenance(citations)
}

fn validate_memory_faded(value: &MemoryFaded) -> Result<(), Error> {
    if !bounded_identifier(&value.memory_id)
        || value
            .evidence_lsns
            .as_ref()
            .is_some_and(|lsns| lsns.is_empty() || lsns.contains(&0))
    {
        Err(Error::new(ErrorCode::SchemaInvalid))
    } else {
        Ok(())
    }
}

fn validate_edge_asserted(value: &EdgeAsserted) -> Result<(), Error> {
    if !bounded_identifier(&value.edge_id)
        || !bounded_identifier(&value.source_id)
        || !bounded_identifier(&value.target_id)
        || value.source_id == value.target_id
        || value.relation.is_empty()
        || value.relation.len() > MAXIMUM_IDENTIFIER_BYTES
        || value.weight_micros == 0
        || value.weight_micros > 1_000_000
        || (value.valid_to_ns != 0 && value.valid_to_ns < value.valid_from_ns)
    {
        return Err(Error::new(ErrorCode::SchemaInvalid));
    }
    validate_provenance(&value.citations)
}

fn validate_edge_retracted(value: &EdgeRetracted) -> Result<(), Error> {
    if !bounded_identifier(&value.edge_id) {
        return Err(Error::new(ErrorCode::SchemaInvalid));
    }
    validate_provenance(&value.citations)
}

fn validate_consolidation_opened(value: &ConsolidationOpened) -> Result<(), Error> {
    if value.scope_digest.len() != 32
        || value.cadence_key.is_empty()
        || value.cadence_key.len() > MAXIMUM_IDENTIFIER_BYTES
        || value.generation == 0
        || value.phases.is_empty()
        || (value.prompts.is_empty()
            && !value
                .phases
                .iter()
                .all(|phase| *phase == ConsolidationPhaseName::Extract))
        || value.prompts.iter().any(|prompt| {
            prompt.prompt_id.is_empty()
                || prompt.prompt_id.len() > MAXIMUM_IDENTIFIER_BYTES
                || prompt.version == 0
                || prompt.model_id.is_empty()
                || prompt.model_id.len() > MAXIMUM_IDENTIFIER_BYTES
        })
        || value.budget.max_llm_calls == 0
        || value.budget.max_tokens == 0
        || value.budget.max_microusd == 0
        || value.budget.max_wall_ms == 0
        || (value.source_first_lsn == 0) != (value.source_last_lsn == 0)
        || value.source_first_lsn > value.source_last_lsn
    {
        Err(Error::new(ErrorCode::SchemaInvalid))
    } else {
        Ok(())
    }
}

fn validate_consolidation_phase(value: &ConsolidationPhase) -> Result<(), Error> {
    if !bounded_identifier(&value.attempt_prefix)
        || value.cursor.as_ref().is_some_and(Vec::is_empty)
        || (value.llm_calls == 0
            && (value.input_tokens != 0 || value.output_tokens != 0 || value.cost_microusd != 0))
    {
        Err(Error::new(ErrorCode::SchemaInvalid))
    } else {
        Ok(())
    }
}

fn validate_consolidation_closed(value: &ConsolidationClosed) -> Result<(), Error> {
    if value.generation == 0
        || (value.llm_calls == 0
            && (value.input_tokens != 0 || value.output_tokens != 0 || value.cost_microusd != 0))
    {
        Err(Error::new(ErrorCode::SchemaInvalid))
    } else {
        Ok(())
    }
}

fn validate_consolidation_retracted(value: &ConsolidationRetracted) -> Result<(), Error> {
    if !bounded_identifier(&value.target_run_id)
        || value.reason.is_empty()
        || value.reason.len() > MAXIMUM_IDENTIFIER_BYTES
    {
        Err(Error::new(ErrorCode::SchemaInvalid))
    } else {
        Ok(())
    }
}

fn validate_reviewed(value: &Reviewed) -> Result<(), Error> {
    if !bounded_identifier(&value.memory_id)
        || value.source_lsn == 0
        || value.reviewed_at_ns <= 0
        || value.stability_millis == 0
        || value.difficulty_micros > 1_000_000
        || value.due_at_ns < value.reviewed_at_ns
    {
        Err(Error::new(ErrorCode::SchemaInvalid))
    } else {
        Ok(())
    }
}

fn validate_belief_fields(
    belief_id: &[u8],
    canonical_identity: &str,
    value: &[u8],
    valid_from_ns: i64,
    valid_to_ns: i64,
    provenance: &[ProvenanceRange],
    conflict_domain: Option<&str>,
) -> Result<(), Error> {
    if !bounded_identifier(belief_id)
        || canonical_identity.is_empty()
        || canonical_identity.len() > MAXIMUM_IDENTIFIER_BYTES
        || value.is_empty()
        || value.len() > MAXIMUM_EVENT_BYTES
        || (valid_to_ns != 0 && valid_to_ns < valid_from_ns)
        || conflict_domain
            .is_some_and(|domain| domain.is_empty() || domain.len() > MAXIMUM_IDENTIFIER_BYTES)
    {
        return Err(Error::new(ErrorCode::SchemaInvalid));
    }
    validate_provenance(provenance)
}

fn validate_provenance(provenance: &[ProvenanceRange]) -> Result<(), Error> {
    if provenance.is_empty()
        || provenance.iter().any(|range| {
            range.first_lsn == 0
                || range.last_lsn < range.first_lsn
                || range.byte_end <= range.byte_start
        })
    {
        Err(Error::new(ErrorCode::SchemaInvalid))
    } else {
        Ok(())
    }
}

fn require_prior_tool_call(lsn: u64, history: &impl EventHistory) -> Result<(), Error> {
    let referenced_lsn = LSN::new(lsn);
    if history.kind_at(referenced_lsn) != Some(EventKind::ToolCall) {
        return Err(Error::new(ErrorCode::OrderingViolation).at_lsn(referenced_lsn));
    }
    Ok(())
}

fn bounded_identifier(value: &[u8]) -> bool {
    !value.is_empty() && value.len() <= MAXIMUM_IDENTIFIER_BYTES
}

fn payload_kind(payload: &EventPayload) -> EventKind {
    match payload {
        EventPayload::UserMsg(_) => EventKind::UserMsg,
        EventPayload::DeliveredMsg(_) => EventKind::DeliveredMsg,
        EventPayload::ToolCall(_) => EventKind::ToolCall,
        EventPayload::ToolResult(_) => EventKind::ToolResult,
        EventPayload::Reasoning(_) => EventKind::Reasoning,
        EventPayload::ProviderFrame(_) => EventKind::ProviderFrame,
        EventPayload::MediaRef(_) => EventKind::MediaRef,
        EventPayload::Effect(_) => EventKind::Effect,
        EventPayload::Approval(_) => EventKind::Approval,
        EventPayload::Outcome(_) => EventKind::Outcome,
        EventPayload::Checkpoint(_) => EventKind::Checkpoint,
        EventPayload::Supervisor(_) => EventKind::Supervisor,
        EventPayload::Recovery(_) => EventKind::Recovery,
        EventPayload::IntentSet(_) => EventKind::IntentSet,
        EventPayload::LoopOpened(_) => EventKind::LoopOpened,
        EventPayload::LoopClosed(_) => EventKind::LoopClosed,
        EventPayload::Assertion(_) => EventKind::Assertion,
        EventPayload::Consolidation(_) => EventKind::Consolidation,
        EventPayload::Embedding(_) => EventKind::Embedding,
        EventPayload::Retract(_) => EventKind::Retract,
        EventPayload::Attestation(_) => EventKind::Attestation,
        EventPayload::Binding(_) => EventKind::Binding,
        EventPayload::ProposedAssertion(_) => EventKind::ProposedAssertion,
        EventPayload::MemoryMinted(_) => EventKind::MemoryMinted,
        EventPayload::MemoryRevised(_) => EventKind::MemoryRevised,
        EventPayload::MemoryMerged(_) => EventKind::MemoryMerged,
        EventPayload::MemoryFaded(_) => EventKind::MemoryFaded,
        EventPayload::EdgeAsserted(_) => EventKind::EdgeAsserted,
        EventPayload::EdgeRetracted(_) => EventKind::EdgeRetracted,
        EventPayload::ConsolidationOpened(_) => EventKind::ConsolidationOpened,
        EventPayload::ConsolidationPhase(_) => EventKind::ConsolidationPhase,
        EventPayload::ConsolidationClosed(_) => EventKind::ConsolidationClosed,
        EventPayload::ConsolidationRetracted(_) => EventKind::ConsolidationRetracted,
        EventPayload::Reviewed(_) => EventKind::Reviewed,
        EventPayload::IntentionSet(_) => EventKind::IntentionSet,
        EventPayload::IntentionFired(_) => EventKind::IntentionFired,
        EventPayload::AttentionDecided(_) => EventKind::AttentionDecided,
        EventPayload::IntentionCancelled(_) => EventKind::IntentionCancelled,
        EventPayload::Predicted(_) => EventKind::Predicted,
        EventPayload::OutcomeObserved(_) => EventKind::OutcomeObserved,
        EventPayload::ProcedureMined(_) => EventKind::ProcedureMined,
        EventPayload::ProcedureRevised(_) => EventKind::ProcedureRevised,
        EventPayload::ProcedureAdopted(_) => EventKind::ProcedureAdopted,
        EventPayload::VocabularyImported(_) => EventKind::VocabularyImported,
        EventPayload::DocumentIngested(_) => EventKind::DocumentIngested,
        EventPayload::DocumentExtracted(_) => EventKind::DocumentExtracted,
        EventPayload::DocumentChunked(_) => EventKind::DocumentChunked,
        EventPayload::SourceConnectorBound(_) => EventKind::SourceConnectorBound,
        EventPayload::SourceDeliveryAccepted(_) => EventKind::SourceDeliveryAccepted,
        EventPayload::SourceDeliverySettled(_) => EventKind::SourceDeliverySettled,
        EventPayload::SourceRevisionObserved(_) => EventKind::SourceRevisionObserved,
        EventPayload::ProcedureImported(_) => EventKind::ProcedureImported,
        EventPayload::ProcedureImprovementProposed(_) => EventKind::ProcedureImprovementProposed,
    }
}
