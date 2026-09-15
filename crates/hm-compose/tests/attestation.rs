mod common;

use hm_compose::bundle::{ActivationRequest, AttestationRequest, activate, build_attestations};
use hm_compose::tokens::{FallbackWeights, TokenCounter};
use hm_core::{ActorId, LSN, UtcNanos};
use hm_ledger::frame::EventKind;
use hm_proj::rebuild::rebuild_projection_stream;
use hm_proj::store::ProjectionStore;
use hm_schema::event::{Boundary, EventKind as SchemaEventKind, verify_event};
use hm_schema::events::AttestationDisposition;

#[test]
fn every_unique_provenance_lsn_becomes_a_valid_attestation_frame() {
    let temporary = tempfile::tempdir().expect("temporary directory");
    let store = ProjectionStore::open(temporary.path(), 16 * 1024 * 1024).expect("store");
    let (conversation, frames) = common::workload();
    rebuild_projection_stream(&store, &frames, true, usize::MAX).expect("projections");
    let snapshot = store.begin_snapshot().expect("snapshot");
    let counter =
        TokenCounter::for_model("fallback", None, FallbackWeights::default()).expect("counter");
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
    .expect("activate");
    let attestations = build_attestations(
        &mut bundle,
        AttestationRequest {
            first_lsn: LSN::new(100),
            actor: ActorId::new(19),
            conversation,
            wall_timestamp_ns: UtcNanos::new(999),
            disposition: AttestationDisposition::Used,
        },
    )
    .expect("attestations");
    assert_eq!(attestations.len(), 4);
    assert_eq!(bundle.manifest.used.len(), 4);
    for (index, frame) in attestations.iter().enumerate() {
        assert_eq!(frame.header.kind, EventKind::Attestation);
        assert_eq!(
            frame.header.lsn,
            LSN::new(100 + u64::try_from(index).expect("index"))
        );
        verify_event(
            &frame.sealed_payload,
            SchemaEventKind::Attestation,
            Boundary::Socket,
        )
        .expect("valid attestation event");
    }
}
