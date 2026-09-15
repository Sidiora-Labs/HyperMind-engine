use hm_core::{ErrorCode, LSN};
use hm_schema::event::{
    Boundary, EventKind, encode_event_envelope, verify_event, verify_event_with_history,
};
use hm_schema::events::{
    Attestation, AttestationDisposition, Authority, DeliveredMsg, EventEnvelope, EventPayload,
    Reasoning, Retention, Sensitivity, ToolCall, ToolResult, UserMsg,
};
use hm_schema::protocol::{
    CURRENT_PROTOCOL_VERSION, encode_wire_envelope, validate_request, verify_request,
    verify_wire_envelope,
};
use hm_schema::wire::{
    Activate, Health, Request, RequestPayload, Stats, WireEnvelope, WirePayload,
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
    unknown[tag] = 22;
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
