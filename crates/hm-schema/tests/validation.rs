use hm_core::{ErrorCode, LSN};
use hm_schema::event::{
    Boundary, EventHistory, EventKind, HistorySource, encode_event_envelope, verify_event,
    verify_event_with_history,
};
use hm_schema::events::{
    Approval, Assertion, AssertionClaim, Attestation, AttestationDisposition, Authority,
    BeliefType, Binding, Checkpoint, Consolidation, ConsolidationBudget, ConsolidationClosed,
    ConsolidationOpened, ConsolidationPhase, ConsolidationPhaseName, ConsolidationPhaseState,
    ConsolidationRetracted, DeliveredMsg, EdgeAsserted, EdgeRetracted, Effect, Embedding,
    EventEnvelope, EventPayload, IntentSet, LoopCloseReason, LoopClosed, LoopOpened,
    MemoryFadeReason, MemoryFaded, MemoryId, MemoryMerged, MemoryMinted, MemoryRevised,
    ModelProvenance, Outcome, PromptVersion, ProposedAssertion, ProvenanceRange, Reasoning,
    Recovery, Retention, Retract, ReviewRating, Reviewed, Sensitivity, Supervisor, ToolCall,
    ToolResult, UserMsg,
};
use hm_schema::protocol::{
    CURRENT_PROTOCOL_VERSION, encode_wire_envelope, validate_request, verify_request,
    verify_wire_envelope,
};
use hm_schema::wire::{
    Activate, AsOf, Checkpoint as CheckpointRequest, Event as WireEvent, Health, LatestCheckpoint,
    Request, RequestPayload, Stats, Subscribe, WireEnvelope, WirePayload,
};
use std::fs;

fn event_envelope(payload: EventPayload, schema_version: u16) -> EventEnvelope {
    EventEnvelope {
        schema_version,
        payload,
        connection_id: None,
        client_seq: 0,
        client_event_index: 0,
        client_event_count: 0,
        origin_actor: 0,
        run_id: None,
        model_provenance: None,
        authority: Authority::UserAsserted,
        retention: Retention::CurrentState,
        sensitivity: Sensitivity::Public,
        event_time_ns: 0,
    }
}

fn encode_event(envelope: &EventEnvelope) -> Vec<u8> {
    encode_event_envelope(envelope)
}

fn encode_wire(envelope: &WireEnvelope) -> Vec<u8> {
    encode_wire_envelope(envelope)
}

struct OneHistory {
    lsn: LSN,
    kind: EventKind,
    authority: Authority,
    source: HistorySource,
}

impl EventHistory for OneHistory {
    fn kind_at(&self, lsn: LSN) -> Option<EventKind> {
        (lsn == self.lsn).then_some(self.kind)
    }

    fn authority_at(&self, lsn: LSN) -> Option<Authority> {
        (lsn == self.lsn).then_some(self.authority)
    }

    fn source_at(&self, lsn: LSN) -> HistorySource {
        if lsn == self.lsn {
            self.source
        } else {
            HistorySource::LedgerEvent
        }
    }
}

fn vtable_position(buffer: &[u8], table_position: usize) -> usize {
    let distance = i32::from_le_bytes(
        buffer[table_position..table_position + 4]
            .try_into()
            .expect("table header"),
    );
    usize::try_from(table_position.cast_signed() - isize::try_from(distance).expect("distance"))
        .expect("vtable position")
}

fn root_table_position(buffer: &[u8]) -> usize {
    u32::from_le_bytes(buffer[..4].try_into().expect("root offset")) as usize
}

fn set_table_field_offset(buffer: &mut [u8], table_position: usize, field: usize, value: u16) {
    let position = vtable_position(buffer, table_position) + 4 + field * 2;
    buffer[position..position + 2].copy_from_slice(&value.to_le_bytes());
}

fn table_field_position(buffer: &[u8], table_position: usize, field: usize) -> usize {
    let vtable = vtable_position(buffer, table_position);
    let entry = vtable + 4 + field * 2;
    let offset = u16::from_le_bytes(buffer[entry..entry + 2].try_into().expect("field offset"));
    table_position + usize::from(offset)
}

#[test]
fn verifies_every_wave_one_event_kind() {
    let cases = [
        (
            EventKind::UserMsg,
            EventPayload::UserMsg(Box::new(UserMsg {
                content: b"user".to_vec(),
            })),
        ),
        (
            EventKind::DeliveredMsg,
            EventPayload::DeliveredMsg(Box::new(DeliveredMsg {
                content: b"delivered".to_vec(),
            })),
        ),
        (
            EventKind::Reasoning,
            EventPayload::Reasoning(Box::new(Reasoning {
                content: b"reason".to_vec(),
            })),
        ),
        (
            EventKind::ToolCall,
            EventPayload::ToolCall(Box::new(ToolCall {
                call_id: b"call-1".to_vec(),
                tool_name: "read".to_owned(),
                arguments: Vec::new(),
            })),
        ),
        (
            EventKind::Attestation,
            EventPayload::Attestation(Box::new(Attestation {
                target_lsn: 1,
                disposition: AttestationDisposition::Used,
            })),
        ),
        (
            EventKind::Attestation,
            EventPayload::Attestation(Box::new(Attestation {
                target_lsn: 1,
                disposition: AttestationDisposition::Helpful,
            })),
        ),
        (
            EventKind::Attestation,
            EventPayload::Attestation(Box::new(Attestation {
                target_lsn: 1,
                disposition: AttestationDisposition::Harmful,
            })),
        ),
    ];
    for (kind, payload) in cases {
        let encoded = encode_event(&event_envelope(payload, 2));
        let verified = verify_event(&encoded, kind, Boundary::Socket).expect("valid event");
        assert_eq!(verified.kind, kind);
    }
}

#[test]
fn tool_result_requires_a_prior_tool_call() {
    let encoded = encode_event(&event_envelope(
        EventPayload::ToolResult(Box::new(ToolResult {
            call_id: b"call-1".to_vec(),
            tool_call_lsn: 7,
            result: b"result".to_vec(),
            ..ToolResult::default()
        })),
        2,
    ));
    let error = verify_event(&encoded, EventKind::ToolResult, Boundary::Disk)
        .expect_err("history is required");
    assert_eq!(error.code, ErrorCode::OrderingViolation);

    let verified =
        verify_event_with_history(&encoded, EventKind::ToolResult, Boundary::Disk, &|lsn| {
            (lsn == LSN::new(7)).then_some(EventKind::ToolCall)
        })
        .expect("prior tool call exists");
    assert_eq!(verified.kind, EventKind::ToolResult);
}

#[test]
fn verifies_continuity_events_and_binding_contract() {
    let cases = [
        (
            EventKind::Approval,
            EventPayload::Approval(Box::new(Approval {
                effect_id: b"effect-1".to_vec(),
                ..Approval::default()
            })),
        ),
        (
            EventKind::Checkpoint,
            EventPayload::Checkpoint(Box::new(Checkpoint {
                cursor: b"opaque-turn-state".to_vec(),
            })),
        ),
        (
            EventKind::Supervisor,
            EventPayload::Supervisor(Box::new(Supervisor {
                code: "resume".to_owned(),
                evidence: Vec::new(),
            })),
        ),
        (
            EventKind::Recovery,
            EventPayload::Recovery(Box::new(Recovery {
                code: "reconcile".to_owned(),
                target_lsn: 7,
            })),
        ),
        (
            EventKind::IntentSet,
            EventPayload::IntentSet(Box::new(IntentSet {
                objective: b"finish continuity slice".to_vec(),
            })),
        ),
        (
            EventKind::LoopOpened,
            EventPayload::LoopOpened(Box::new(LoopOpened {
                loop_id: b"loop-1".to_vec(),
                objective: b"dispatch tool".to_vec(),
            })),
        ),
        (
            EventKind::LoopClosed,
            EventPayload::LoopClosed(Box::new(LoopClosed {
                loop_id: b"loop-1".to_vec(),
                reason: LoopCloseReason::Abandoned,
                cause: b"cancelled".to_vec(),
                evidence_lsns: None,
            })),
        ),
        (
            EventKind::Binding,
            EventPayload::Binding(Box::new(Binding {
                task: Some(b"task-1".to_vec()),
                scope: None,
                canonical_entity: "src/main.rs".to_owned(),
                property: "revision".to_owned(),
                evidence_lsn: 9,
                revision: b"abc123".to_vec(),
                freshness_requirement_ns: 60_000_000_000,
            })),
        ),
    ];
    for (kind, payload) in cases {
        let encoded = encode_event(&event_envelope(payload, 2));
        assert_eq!(
            verify_event(&encoded, kind, Boundary::Socket)
                .expect("valid continuity event")
                .kind,
            kind
        );
    }

    let effect = encode_event(&event_envelope(
        EventPayload::Effect(Box::new(Effect {
            effect_id: b"effect-1".to_vec(),
            tool_call_lsn: 7,
            ..Effect::default()
        })),
        2,
    ));
    assert_eq!(
        verify_event_with_history(&effect, EventKind::Effect, Boundary::Socket, &|lsn| {
            (lsn == LSN::new(7)).then_some(EventKind::ToolCall)
        })
        .expect("effect cites prior tool call")
        .kind,
        EventKind::Effect
    );
}

#[test]
fn binding_requires_exactly_one_task_or_scope() {
    let encoded = encode_event(&event_envelope(
        EventPayload::Binding(Box::new(Binding {
            task: Some(b"task-1".to_vec()),
            scope: Some(b"scope-1".to_vec()),
            canonical_entity: "src/main.rs".to_owned(),
            property: "revision".to_owned(),
            evidence_lsn: 9,
            revision: b"abc123".to_vec(),
            freshness_requirement_ns: 1,
        })),
        2,
    ));
    assert_eq!(
        verify_event(&encoded, EventKind::Binding, Boundary::Socket)
            .expect_err("binding cannot target both task and scope")
            .code,
        ErrorCode::SchemaInvalid
    );
}

#[test]
fn embedding_requires_a_matching_declared_dimension() {
    let embedding =
        |target_lsn, space_id: &str, dimension, quantized: Vec<i8>, binary_prefilter: Vec<u8>| {
            encode_event(&event_envelope(
                EventPayload::Embedding(Box::new(Embedding {
                    target_lsn,
                    dimension,
                    quantized,
                    binary_prefilter,
                    space_id: space_id.to_owned(),
                })),
                2,
            ))
        };
    let space_id = "nomic-v1.5:revision:768:cosine:l2:document";
    let valid = embedding(7, space_id, 9, vec![1; 9], vec![0xff, 0x01]);
    assert_eq!(
        verify_event(&valid, EventKind::Embedding, Boundary::Socket)
            .expect("embedding dimension matches its declared space")
            .kind,
        EventKind::Embedding
    );

    for invalid in [
        embedding(7, space_id, 8, vec![1; 7], vec![0xff]),
        embedding(7, space_id, 8, vec![1; 8], vec![0xff, 0x00]),
        embedding(7, space_id, 0, Vec::new(), Vec::new()),
        embedding(0, space_id, 8, vec![1; 8], vec![0xff]),
        embedding(7, "", 8, vec![1; 8], vec![0xff]),
    ] {
        assert_eq!(
            verify_event(&invalid, EventKind::Embedding, Boundary::Socket)
                .expect_err("embedding dimensions must match")
                .code,
            ErrorCode::SchemaInvalid
        );
    }
}

#[test]
fn completed_outcomes_and_loops_require_observed_ledger_evidence() {
    let outcome = |evidence_lsns| {
        encode_event(&event_envelope(
            EventPayload::Outcome(Box::new(Outcome {
                effect_id: b"effect-1".to_vec(),
                detail: b"written".to_vec(),
                evidence_lsns,
                ..Outcome::default()
            })),
            2,
        ))
    };
    let done = encode_event(&event_envelope(
        EventPayload::LoopClosed(Box::new(LoopClosed {
            loop_id: b"loop-1".to_vec(),
            reason: LoopCloseReason::Done,
            cause: b"effect observed".to_vec(),
            evidence_lsns: Some(vec![7]),
        })),
        2,
    ));
    for authority in [
        Authority::ToolObserved,
        Authority::ExternalObserved,
        Authority::RuntimeFact,
    ] {
        let history = OneHistory {
            lsn: LSN::new(7),
            kind: EventKind::ToolResult,
            authority,
            source: HistorySource::LedgerEvent,
        };
        verify_event_with_history(
            &outcome(Some(vec![7])),
            EventKind::Outcome,
            Boundary::Socket,
            &history,
        )
        .expect("observed outcome evidence");
        verify_event_with_history(&done, EventKind::LoopClosed, Boundary::Socket, &history)
            .expect("observed loop evidence");
    }

    assert_eq!(
        verify_event(&outcome(None), EventKind::Outcome, Boundary::Socket)
            .expect_err("outcome evidence is required")
            .code,
        ErrorCode::CitationInvalid
    );
    let assistant_history = OneHistory {
        lsn: LSN::new(7),
        kind: EventKind::Reasoning,
        authority: Authority::AssistantGenerated,
        source: HistorySource::LedgerEvent,
    };
    assert_eq!(
        verify_event_with_history(
            &outcome(Some(vec![7])),
            EventKind::Outcome,
            Boundary::Socket,
            &assistant_history,
        )
        .expect_err("assistant text is not outcome evidence")
        .code,
        ErrorCode::CitationInvalid
    );
    for source in [
        HistorySource::Memory,
        HistorySource::Summary,
        HistorySource::Reconstruction,
    ] {
        let laundered = OneHistory {
            lsn: LSN::new(7),
            kind: EventKind::ToolResult,
            authority: Authority::ToolObserved,
            source,
        };
        assert_eq!(
            verify_event_with_history(&done, EventKind::LoopClosed, Boundary::Socket, &laundered,)
                .expect_err("memory-derived evidence is rejected")
                .code,
            ErrorCode::CitationInvalid
        );
    }
}

#[test]
fn event_validation_fails_closed() {
    let future = encode_event(&event_envelope(
        EventPayload::UserMsg(Box::new(UserMsg {
            content: b"future".to_vec(),
        })),
        3,
    ));
    assert_eq!(
        verify_event(&future, EventKind::UserMsg, Boundary::Import)
            .expect_err("future version")
            .code,
        ErrorCode::SchemaVersion
    );

    let mut unknown = encode_event(&event_envelope(
        EventPayload::UserMsg(Box::new(UserMsg {
            content: b"unknown".to_vec(),
        })),
        2,
    ));
    let root = root_table_position(&unknown);
    let tag = table_field_position(&unknown, root, 1);
    unknown[tag] = 23;
    assert_eq!(
        verify_event(&unknown, EventKind::UserMsg, Boundary::Import)
            .expect_err("unknown union member")
            .code,
        ErrorCode::SchemaInvalid
    );

    let mut missing = encode_event(&event_envelope(
        EventPayload::UserMsg(Box::new(UserMsg {
            content: b"missing".to_vec(),
        })),
        2,
    ));
    let root = root_table_position(&missing);
    set_table_field_offset(&mut missing, root, 2, 0);
    assert_eq!(
        verify_event(&missing, EventKind::UserMsg, Boundary::Import)
            .expect_err("required payload")
            .code,
        ErrorCode::SchemaInvalid
    );
}

#[test]
fn v1_go_envelope_decodes_with_v2_defaults() {
    let encoded = include_bytes!("../fixtures/schema/schema_user_msg.bin");
    let verified = verify_event(encoded, EventKind::UserMsg, Boundary::Import)
        .expect("v1 Go-compatible fixture");
    assert_eq!(verified.envelope.schema_version, 1);
    assert_eq!(verified.envelope.origin_actor, 0);
    assert_eq!(verified.envelope.run_id, None);
    assert_eq!(verified.envelope.model_provenance, None);
    assert_eq!(verified.envelope.authority, Authority::UserAsserted);
    assert_eq!(verified.envelope.retention, Retention::CurrentState);
    assert_eq!(verified.envelope.sensitivity, Sensitivity::Public);
    assert_eq!(verified.envelope.event_time_ns, 0);
}

fn health_request(version: u16) -> WireEnvelope {
    WireEnvelope {
        proto_version: version,
        payload: WirePayload::Request(Box::new(Request {
            request_id: 9,
            payload: RequestPayload::Health(Box::new(Health {})),
        })),
    }
}

#[test]
fn request_validation_accepts_v2_and_v3_and_rejects_unknown_versions() {
    for version in [2, CURRENT_PROTOCOL_VERSION] {
        let encoded = encode_wire(&health_request(version));
        let verified = verify_request(&encoded).expect("compatible request");
        assert_eq!(verified.proto_version, version);
    }
    let encoded = encode_wire(&health_request(4));
    assert_eq!(
        verify_request(&encoded).expect_err("future protocol").code,
        ErrorCode::ProtocolVersion
    );
}

#[test]
fn protocol_validation_rejects_unknown_union_and_missing_payload() {
    let mut unknown = encode_wire(&health_request(CURRENT_PROTOCOL_VERSION));
    let root = root_table_position(&unknown);
    let tag = table_field_position(&unknown, root, 1);
    unknown[tag] = 6;
    assert_eq!(
        verify_wire_envelope(&unknown)
            .expect_err("unknown wire union")
            .code,
        ErrorCode::ProtocolInvalid
    );

    let mut missing = encode_wire(&health_request(CURRENT_PROTOCOL_VERSION));
    let root = root_table_position(&missing);
    set_table_field_offset(&mut missing, root, 2, 0);
    assert_eq!(
        verify_wire_envelope(&missing)
            .expect_err("required wire payload")
            .code,
        ErrorCode::ProtocolInvalid
    );
}

#[test]
fn request_semantics_are_bounded_and_wave_scoped() {
    let invalid_activate = Request {
        request_id: 1,
        payload: RequestPayload::Activate(Box::new(Activate {
            conversation: vec![0; 16],
            query: Vec::new(),
            budget_tokens: 100,
            token_weights: Some(vec![1; 255]),
            ..Activate::default()
        })),
    };
    assert_eq!(
        validate_request(&invalid_activate)
            .expect_err("token table size")
            .code,
        ErrorCode::ProtocolInvalid
    );

    let unavailable = Request {
        request_id: 2,
        payload: RequestPayload::Stats(Box::new(Stats { actor: 0 })),
    };
    assert_eq!(
        validate_request(&unavailable).expect_err("actor zero").code,
        ErrorCode::ProtocolInvalid
    );
}

#[test]
fn continuity_protocol_requests_and_event_push_are_enabled() {
    for payload in [
        RequestPayload::Checkpoint(Box::new(CheckpointRequest {
            turn_id: b"turn-7".to_vec(),
            blob: b"opaque checkpoint".to_vec(),
            client_seq: 3,
        })),
        RequestPayload::LatestCheckpoint(Box::new(LatestCheckpoint {
            turn_id: b"turn-7".to_vec(),
        })),
        RequestPayload::Subscribe(Box::new(Subscribe {
            conversation: Some(vec![0x17; 16]),
            since_lsn: 9,
        })),
    ] {
        validate_request(&Request {
            request_id: 7,
            payload,
        })
        .expect("continuity request");
    }

    let pushed = WireEnvelope {
        proto_version: CURRENT_PROTOCOL_VERSION,
        payload: WirePayload::Event(Box::new(WireEvent {
            subscription_id: 1,
            lsn: 11,
            kind: EventKind::Binding as u8,
            wall_timestamp_ns: 1_000,
            actor: 7,
            conversation: vec![0x17; 16],
            payload: b"sealed event".to_vec(),
        })),
    };
    assert!(verify_wire_envelope(&encode_wire(&pushed)).is_ok());

    let invalid = Request {
        request_id: 8,
        payload: RequestPayload::Checkpoint(Box::new(CheckpointRequest {
            turn_id: Vec::new(),
            blob: Vec::new(),
            client_seq: 0,
        })),
    };
    assert_eq!(
        validate_request(&invalid)
            .expect_err("invalid checkpoint")
            .code,
        ErrorCode::ProtocolInvalid
    );
}

fn belief_provenance() -> Vec<ProvenanceRange> {
    vec![ProvenanceRange {
        first_lsn: 7,
        last_lsn: 9,
        byte_start: 4,
        byte_end: 18,
    }]
}

fn belief_assertion() -> Assertion {
    Assertion {
        belief_id: b"belief-1".to_vec(),
        belief_type: BeliefType::Fact,
        canonical_identity: "deployment:region:europe".to_owned(),
        value: b"eu-central".to_vec(),
        valid_from_ns: 100,
        valid_to_ns: 200,
        provenance: belief_provenance(),
        conflict_domain: Some("deployment:region".to_owned()),
        claim: AssertionClaim::Affirmative,
    }
}

fn model_provenance() -> ModelProvenance {
    ModelProvenance {
        model_id: "fixture-model".to_owned(),
        prompt_id: "merge-cluster".to_owned(),
        prompt_version: 1,
        temperature: 0.2,
        call_id: Some(b"call-7".to_vec()),
        input_tokens: 120,
        output_tokens: 30,
        cache_read_tokens: 20,
        cache_write_tokens: 0,
        cost_microusd: 17,
    }
}

fn memory_minted() -> MemoryMinted {
    MemoryMinted {
        memory_id: b"memory-1".to_vec(),
        name: "Deployment region".to_owned(),
        definition: b"The service is deployed in eu-central.".to_vec(),
        tags: vec!["deployment".to_owned(), "region".to_owned()],
        salience_micros: 700_000,
        citations: belief_provenance(),
    }
}

fn wave_six_envelope(payload: EventPayload, llm_derived: bool) -> EventEnvelope {
    let mut envelope = event_envelope(payload, 2);
    envelope.run_id = Some(b"run-6".to_vec());
    envelope.authority = Authority::DerivedInference;
    if llm_derived {
        envelope.model_provenance = Some(Box::new(model_provenance()));
    }
    envelope
}

#[test]
#[allow(clippy::too_many_lines)]
fn consolidation_events_round_trip_with_generation_and_model_provenance() {
    let minted = memory_minted();
    let cases = [
        (
            EventKind::MemoryMinted,
            EventPayload::MemoryMinted(Box::new(minted.clone())),
            true,
        ),
        (
            EventKind::MemoryRevised,
            EventPayload::MemoryRevised(Box::new(MemoryRevised {
                memory_id: minted.memory_id.clone(),
                previous_lsn: 21,
                name: minted.name.clone(),
                definition: b"The service remains in eu-central.".to_vec(),
                tags: minted.tags.clone(),
                salience_micros: 750_000,
                citations: belief_provenance(),
            })),
            true,
        ),
        (
            EventKind::MemoryMerged,
            EventPayload::MemoryMerged(Box::new(MemoryMerged {
                memory_id: b"memory-merged".to_vec(),
                merged_memory_ids: vec![
                    MemoryId {
                        value: b"memory-1".to_vec(),
                    },
                    MemoryId {
                        value: b"memory-2".to_vec(),
                    },
                ],
                name: "Deployment".to_owned(),
                definition: b"The service deployment is in eu-central.".to_vec(),
                tags: vec!["deployment".to_owned()],
                salience_micros: 800_000,
                citations: belief_provenance(),
            })),
            true,
        ),
        (
            EventKind::MemoryFaded,
            EventPayload::MemoryFaded(Box::new(MemoryFaded {
                memory_id: minted.memory_id.clone(),
                reason: MemoryFadeReason::LowRetrievability,
                evidence_lsns: Some(vec![31]),
            })),
            false,
        ),
        (
            EventKind::EdgeAsserted,
            EventPayload::EdgeAsserted(Box::new(EdgeAsserted {
                edge_id: b"edge-1".to_vec(),
                source_id: b"memory-1".to_vec(),
                target_id: b"memory-2".to_vec(),
                relation: "supports".to_owned(),
                weight_micros: 500_000,
                valid_from_ns: 100,
                valid_to_ns: 0,
                citations: belief_provenance(),
            })),
            true,
        ),
        (
            EventKind::EdgeRetracted,
            EventPayload::EdgeRetracted(Box::new(EdgeRetracted {
                edge_id: b"edge-1".to_vec(),
                citations: belief_provenance(),
            })),
            true,
        ),
        (
            EventKind::ConsolidationOpened,
            EventPayload::ConsolidationOpened(Box::new(ConsolidationOpened {
                scope_digest: vec![0x61; 32],
                cadence_key: "daily:2026-09-16".to_owned(),
                generation: 6,
                expected_active_generation: 5,
                phases: vec![
                    ConsolidationPhaseName::Nrem,
                    ConsolidationPhaseName::Connect,
                ],
                prompts: vec![PromptVersion {
                    prompt_id: "merge-cluster".to_owned(),
                    version: 1,
                    model_id: "fixture-model".to_owned(),
                }],
                budget: Box::new(ConsolidationBudget {
                    max_llm_calls: 8,
                    max_tokens: 8_000,
                    max_microusd: 100_000,
                    max_wall_ms: 30_000,
                }),
            })),
            false,
        ),
        (
            EventKind::ConsolidationPhase,
            EventPayload::ConsolidationPhase(Box::new(ConsolidationPhase {
                phase: ConsolidationPhaseName::Nrem,
                state: ConsolidationPhaseState::Completed,
                attempt_prefix: b"run-6:nrem:1".to_vec(),
                cursor: Some(b"cluster-4".to_vec()),
                llm_calls: 1,
                input_tokens: 120,
                output_tokens: 30,
                cost_microusd: 17,
                dropped_candidates: 0,
            })),
            false,
        ),
        (
            EventKind::ConsolidationClosed,
            EventPayload::ConsolidationClosed(Box::new(ConsolidationClosed {
                generation: 6,
                expected_active_generation: 5,
                derived_records: 3,
                dropped_candidates: 1,
                llm_calls: 1,
                input_tokens: 120,
                output_tokens: 30,
                cost_microusd: 17,
            })),
            false,
        ),
        (
            EventKind::ConsolidationRetracted,
            EventPayload::ConsolidationRetracted(Box::new(ConsolidationRetracted {
                target_run_id: b"run-6".to_vec(),
                previous_generation: 5,
                reason: "operator rollback".to_owned(),
            })),
            false,
        ),
        (
            EventKind::Reviewed,
            EventPayload::Reviewed(Box::new(Reviewed {
                memory_id: minted.memory_id,
                rating: ReviewRating::Good,
                source_lsn: 41,
                reviewed_at_ns: 1_000,
                stability_millis: 86_400_000,
                difficulty_micros: 400_000,
                due_at_ns: 2_000,
            })),
            false,
        ),
    ];
    for (kind, payload, llm_derived) in cases {
        assert_eq!(EventKind::try_from(kind as u8), Ok(kind));
        let encoded = encode_event(&wave_six_envelope(payload, llm_derived));
        let verified = verify_event(&encoded, kind, Boundary::Socket).expect("wave six event");
        assert_eq!(verified.kind, kind);
        assert_eq!(
            verified.envelope.run_id.as_deref(),
            Some(b"run-6".as_slice())
        );
        if llm_derived {
            let model = verified.envelope.model_provenance.unwrap();
            assert_eq!(model.call_id.as_deref(), Some(b"call-7".as_slice()));
            assert_eq!(model.input_tokens, 120);
            assert_eq!(model.cost_microusd, 17);
        }
    }
}

#[test]
fn llm_derived_events_require_run_model_usage_and_byte_range_citations() {
    let payload = EventPayload::MemoryMinted(Box::new(memory_minted()));
    let encoded = encode_event(&event_envelope(payload.clone(), 2));
    assert_eq!(
        verify_event(&encoded, EventKind::MemoryMinted, Boundary::Socket)
            .expect_err("run and model provenance")
            .code,
        ErrorCode::SchemaInvalid
    );

    let mut without_model = event_envelope(payload.clone(), 2);
    without_model.run_id = Some(b"run-6".to_vec());
    assert_eq!(
        verify_event(
            &encode_event(&without_model),
            EventKind::MemoryMinted,
            Boundary::Socket,
        )
        .expect_err("model provenance")
        .code,
        ErrorCode::SchemaInvalid
    );

    let mut invalid_usage = wave_six_envelope(payload, true);
    invalid_usage.model_provenance.as_mut().unwrap().call_id = None;
    assert_eq!(
        verify_event(
            &encode_event(&invalid_usage),
            EventKind::MemoryMinted,
            Boundary::Socket,
        )
        .expect_err("per-call usage identity")
        .code,
        ErrorCode::SchemaInvalid
    );

    let mut invalid_citation = memory_minted();
    invalid_citation.citations[0].byte_end = invalid_citation.citations[0].byte_start;
    let encoded = encode_event(&wave_six_envelope(
        EventPayload::MemoryMinted(Box::new(invalid_citation)),
        true,
    ));
    assert_eq!(
        verify_event(&encoded, EventKind::MemoryMinted, Boundary::Socket)
            .expect_err("nonempty citation byte range")
            .code,
        ErrorCode::SchemaInvalid
    );
}

#[test]
fn belief_events_round_trip_with_tri_temporal_and_byte_provenance_fields() {
    let assertion = belief_assertion();
    let proposed = ProposedAssertion {
        belief_id: assertion.belief_id.clone(),
        belief_type: assertion.belief_type,
        canonical_identity: assertion.canonical_identity.clone(),
        value: assertion.value.clone(),
        valid_from_ns: assertion.valid_from_ns,
        valid_to_ns: assertion.valid_to_ns,
        provenance: assertion.provenance.clone(),
        conflict_domain: assertion.conflict_domain.clone(),
        claim: assertion.claim,
    };
    let cases = [
        (
            EventKind::Assertion,
            EventPayload::Assertion(Box::new(assertion.clone())),
        ),
        (
            EventKind::Consolidation,
            EventPayload::Consolidation(Box::new(Consolidation {
                assertions: vec![assertion],
            })),
        ),
        (
            EventKind::Retract,
            EventPayload::Retract(Box::new(Retract {
                belief_id: b"belief-1".to_vec(),
                provenance: belief_provenance(),
            })),
        ),
        (
            EventKind::ProposedAssertion,
            EventPayload::ProposedAssertion(Box::new(proposed)),
        ),
    ];
    for (kind, payload) in cases {
        let mut envelope = event_envelope(payload, 2);
        envelope.event_time_ns = 123_456;
        let verified = verify_event(&encode_event(&envelope), kind, Boundary::Socket)
            .expect("enabled belief event");
        assert_eq!(verified.kind, kind);
        assert_eq!(verified.envelope.event_time_ns, 123_456);
    }
}

#[test]
fn belief_provenance_requires_a_nonempty_byte_range() {
    for (byte_start, byte_end) in [(0, 0), (8, 8), (9, 8)] {
        let mut assertion = belief_assertion();
        assertion.provenance[0].byte_start = byte_start;
        assertion.provenance[0].byte_end = byte_end;
        let encoded = encode_event(&event_envelope(
            EventPayload::Assertion(Box::new(assertion)),
            2,
        ));
        assert_eq!(
            verify_event(&encoded, EventKind::Assertion, Boundary::Socket)
                .expect_err("empty or reversed byte range")
                .code,
            ErrorCode::SchemaInvalid
        );
    }
}

#[test]
fn asof_requires_exactly_one_valid_or_known_time_axis() {
    for request in [
        AsOf {
            belief_type: BeliefType::Fact as u8,
            canonical_identity: "deployment:region:europe".to_owned(),
            valid_time_ns: 123,
            transaction_lsn: 0,
            known_lsn: 0,
        },
        AsOf {
            belief_type: BeliefType::Fact as u8,
            canonical_identity: "deployment:region:europe".to_owned(),
            valid_time_ns: 0,
            transaction_lsn: 0,
            known_lsn: 17,
        },
    ] {
        validate_request(&Request {
            request_id: 41,
            payload: RequestPayload::AsOf(Box::new(request)),
        })
        .expect("one as-of axis");
    }

    for request in [
        AsOf {
            belief_type: BeliefType::Fact as u8,
            canonical_identity: "deployment:region:europe".to_owned(),
            valid_time_ns: 0,
            transaction_lsn: 0,
            known_lsn: 0,
        },
        AsOf {
            belief_type: BeliefType::Fact as u8,
            canonical_identity: "deployment:region:europe".to_owned(),
            valid_time_ns: 123,
            transaction_lsn: 0,
            known_lsn: 17,
        },
    ] {
        assert_eq!(
            validate_request(&Request {
                request_id: 42,
                payload: RequestPayload::AsOf(Box::new(request)),
            })
            .expect_err("as-of requires exactly one axis")
            .code,
            ErrorCode::ProtocolInvalid
        );
    }
}

#[test]
fn donor_protocol_fuzz_corpus_never_panics() {
    let directory = format!("{}/fixtures/protocol_frame", env!("CARGO_MANIFEST_DIR"));
    let mut count = 0;
    for entry in fs::read_dir(directory).expect("protocol fixture directory") {
        let path = entry.expect("fixture entry").path();
        let bytes = fs::read(path).expect("fixture bytes");
        let _ = verify_wire_envelope(&bytes);
        count += 1;
    }
    assert_eq!(count, 45);
}
