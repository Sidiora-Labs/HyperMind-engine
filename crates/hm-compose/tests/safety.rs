#![forbid(unsafe_code)]

mod common;

use hm_compose::bundle::{ActivationRequest, Tier, activate};
use hm_compose::manifest::{manifest_id, validate_reuse};
use hm_compose::safety::{decode_semantic_content, render};
use hm_compose::tokens::{FallbackWeights, TokenCounter};
use hm_core::{ActorId, ErrorCode, LSN};
use hm_proj::rebuild::rebuild_projection_stream;
use hm_proj::store::ProjectionStore;
use hm_schema::event::{EventKind, encode_event_envelope};
use hm_schema::events::{
    Authority, EventEnvelope, EventPayload, Retention, Sensitivity, ToolCall, UserMsg,
};

fn encoded(payload: EventPayload, authority: Authority) -> Vec<u8> {
    encode_event_envelope(&EventEnvelope {
        schema_version: 2,
        payload,
        connection_id: None,
        client_seq: 0,
        client_event_index: 0,
        client_event_count: 0,
        origin_actor: 0,
        run_id: None,
        model_provenance: None,
        authority,
        retention: Retention::Durable,
        sensitivity: Sensitivity::Personal,
        event_time_ns: 0,
    })
}

#[test]
fn malformed_nonsemantic_and_raw_payloads_decode_to_empty() {
    let user = encoded(
        EventPayload::UserMsg(Box::new(UserMsg {
            content: b"safe semantic bytes".to_vec(),
        })),
        Authority::UserAsserted,
    );
    assert!(decode_semantic_content(&user, EventKind::DeliveredMsg).is_none());
    let tool_call = encoded(
        EventPayload::ToolCall(Box::new(ToolCall {
            call_id: b"call-1".to_vec(),
            tool_name: "read".to_owned(),
            arguments: Vec::new(),
        })),
        Authority::RuntimeFact,
    );
    assert!(decode_semantic_content(&tool_call, EventKind::ToolCall).is_none());
    let raw_envelope = encoded(
        EventPayload::UserMsg(Box::new(UserMsg { content: user })),
        Authority::UserAsserted,
    );
    assert!(decode_semantic_content(&raw_envelope, EventKind::UserMsg).is_none());
    let raw_checkpoint = encoded(
        EventPayload::UserMsg(Box::new(UserMsg {
            content: b"PCCN raw checkpoint".to_vec(),
        })),
        Authority::UserAsserted,
    );
    assert!(decode_semantic_content(&raw_checkpoint, EventKind::UserMsg).is_none());
}

#[test]
fn renderer_excludes_unsafe_items_and_labels_every_item_as_user_content() {
    let temporary = tempfile::tempdir().unwrap();
    let store = ProjectionStore::open(temporary.path(), 16 * 1024 * 1024).unwrap();
    let (conversation, frames) = common::workload();
    rebuild_projection_stream(&store, &frames, true, usize::MAX).unwrap();
    let snapshot = store.begin_snapshot().unwrap();
    let counter = TokenCounter::for_model("fallback", None, FallbackWeights::default()).unwrap();
    let mut bundle = activate(
        &snapshot,
        &ActivationRequest {
            actor: ActorId::new(19),
            conversation,
            query: "alpha".to_owned(),
            turn_text: String::new(),
            budget_tokens: 10_000,
            token_counter: &counter,
            maximum_candidates: 64,
            maximum_conversation_records: 64,
        },
    )
    .unwrap();
    let same_turn = bundle.sections[Tier::Conversation as usize].items[0].provenance[0];
    bundle.sections[Tier::Conversation as usize].items[1]
        .provenance
        .clear();
    let rendered = render(&bundle, [same_turn]).unwrap();
    assert!(rendered.sections.iter().all(|section| {
        section
            .items
            .iter()
            .all(|item| item.role == "user" && !item.provenance.is_empty())
    }));
    assert!(
        rendered
            .sections
            .iter()
            .flat_map(|section| &section.items)
            .all(|item| item.provenance.iter().all(|lsn| *lsn != same_turn))
    );
    assert!(
        rendered
            .sections
            .iter()
            .flat_map(|section| &section.items)
            .any(|item| item.authority == Authority::UserAsserted)
    );
}

#[test]
fn manifest_identity_binds_query_epoch_and_selected_digests() {
    let temporary = tempfile::tempdir().unwrap();
    let store = ProjectionStore::open(temporary.path(), 16 * 1024 * 1024).unwrap();
    let (conversation, frames) = common::workload();
    rebuild_projection_stream(&store, &frames, true, usize::MAX).unwrap();
    let snapshot = store.begin_snapshot().unwrap();
    let counter = TokenCounter::for_model("fallback", None, FallbackWeights::default()).unwrap();
    let mut bundle = activate(
        &snapshot,
        &ActivationRequest {
            actor: ActorId::new(19),
            conversation,
            query: "alpha".to_owned(),
            turn_text: String::new(),
            budget_tokens: 10_000,
            token_counter: &counter,
            maximum_candidates: 64,
            maximum_conversation_records: 64,
        },
    )
    .unwrap();
    let selected: Vec<_> = bundle
        .manifest
        .selected
        .iter()
        .map(|lsn| (*lsn, [u8::try_from(lsn.get()).unwrap(); 32]))
        .collect();
    bundle.manifest.manifest_id = manifest_id(
        &bundle.manifest.query_digest,
        bundle.manifest.snapshot_epoch,
        &selected,
    );
    validate_reuse(&bundle.manifest, bundle.snapshot_epoch, &selected).unwrap();
    assert_eq!(
        validate_reuse(&bundle.manifest, bundle.snapshot_epoch + 1, &selected)
            .unwrap_err()
            .code,
        ErrorCode::ProjectionCheckpoint
    );
    let mut changed = selected;
    changed[0].1[0] ^= 1;
    assert_eq!(
        validate_reuse(&bundle.manifest, bundle.snapshot_epoch, &changed)
            .unwrap_err()
            .code,
        ErrorCode::ProjectionCheckpoint
    );
    assert!(!bundle.manifest.selected.contains(&LSN::new(0)));
}
