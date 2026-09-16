#![allow(clippy::missing_errors_doc)]

use crate::events::{
    Assertion, AttestationDisposition, Authority, Binding, Consolidation, ConsolidationClosed,
    ConsolidationOpened, ConsolidationPhase, ConsolidationRetracted, EdgeAsserted, EdgeRetracted,
    Effect, Embedding, EventEnvelope, EventEnvelopeRef, EventPayload, LoopCloseReason, LoopClosed,
    MemoryFaded, MemoryMerged, MemoryMinted, MemoryRevised, Outcome, ProposedAssertion,
    ProvenanceRange, Retract, Reviewed, ToolResult,
};
use hm_core::{Error, ErrorCode, LSN};
use planus::ReadAsRoot;

use crate::validate::authority::validate_optional_observed_evidence;

pub const CURRENT_SCHEMA_VERSION: u16 = 2;
pub const MAXIMUM_EVENT_BYTES: usize = 16 * 1024 * 1024;
pub const MAXIMUM_IDENTIFIER_BYTES: usize = 4096;

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
    if !expected_kind.is_wave_six() || payload_kind(&envelope.payload) != expected_kind {
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
                    || model.input_tokens.saturating_add(model.output_tokens) == 0)))
    {
        return Err(Error::new(ErrorCode::SchemaInvalid));
    }
    if kind.requires_run_id() && !envelope.run_id.as_deref().is_some_and(bounded_identifier) {
        return Err(Error::new(ErrorCode::SchemaInvalid));
    }
    if kind.is_llm_derived() && envelope.model_provenance.is_none() {
        return Err(Error::new(ErrorCode::SchemaInvalid));
    }
    Ok(())
}

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
    }
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
        || value.prompts.is_empty()
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
    }
}
