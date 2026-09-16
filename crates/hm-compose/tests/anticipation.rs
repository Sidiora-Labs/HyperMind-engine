#![allow(clippy::cast_possible_wrap)]
#![forbid(unsafe_code)]

use hm_compose::bundle::{
    ActivationContext, ActivationRequest, Tier, WhyCode, activate_with_context,
};
use hm_compose::safety;
use hm_compose::tokens::{FallbackWeights, TokenCounter};
use hm_core::{ActorId, ConversationId, LSN, UtcNanos};
use hm_ledger::frame::{EventKind, Frame, FrameHeader};
use hm_proj::rebuild::rebuild_projection_stream;
use hm_proj::store::ProjectionStore;
use hm_schema::event::encode_event_envelope;
use hm_schema::events::{
    AttentionDecided, AttentionDecision, Authority, EventEnvelope, EventPayload,
    IntentionCancelled, IntentionFired, IntentionSet, ProcedureAdopted, ProcedureMined,
    ProcedureSupport, Retention, Sensitivity, UserMsg, WakeRepositoryChanged, WakeTrigger,
};

const CONVERSATION: ConversationId = ConversationId::new([7; 16]);

fn push(
    frames: &mut Vec<Frame>,
    kind: EventKind,
    payload: EventPayload,
    authority: Authority,
) -> u64 {
    let lsn = frames.len() as u64 + 1;
    frames.push(Frame {
        header: FrameHeader {
            lsn: LSN::new(lsn),
            kind,
            wall_timestamp_ns: UtcNanos::new(lsn as i64),
            actor: ActorId::new(7),
            conversation: CONVERSATION,
        },
        sealed_payload: encode_event_envelope(&EventEnvelope {
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
            retention: Retention::CurrentState,
            sensitivity: Sensitivity::Public,
            event_time_ns: lsn as i64,
        }),
    });
    lsn
}

fn attention_frames() -> Vec<Frame> {
    let mut frames = Vec::new();
    let trigger = push(
        &mut frames,
        EventKind::UserMsg,
        EventPayload::UserMsg(Box::new(UserMsg {
            content: b"repository changed".to_vec(),
        })),
        Authority::UserAsserted,
    );
    for (i, decision) in [
        AttentionDecision::Ignore,
        AttentionDecision::Remember,
        AttentionDecision::Schedule,
        AttentionDecision::Notify,
        AttentionDecision::AskUser,
        AttentionDecision::StartWork,
        AttentionDecision::Batch,
        AttentionDecision::Batch,
        AttentionDecision::Notify,
    ]
    .into_iter()
    .enumerate()
    {
        let id = format!("intention-{i}").into_bytes();
        let wake = format!("wake-{i}").into_bytes();
        push(
            &mut frames,
            EventKind::IntentionSet,
            EventPayload::IntentionSet(Box::new(IntentionSet {
                intention_id: id.clone(),
                objective: format!("objective-{i}").into_bytes(),
                trigger: Some(WakeTrigger::WakeRepositoryChanged(Box::new(
                    WakeRepositoryChanged {
                        repository: "repo".to_owned(),
                    },
                ))),
                expires_at_ns: 10_000,
                reply_route: "conversation".to_owned(),
            })),
            Authority::UserAsserted,
        );
        push(
            &mut frames,
            EventKind::IntentionFired,
            EventPayload::IntentionFired(Box::new(IntentionFired {
                intention_id: id.clone(),
                wake_id: wake.clone(),
                trigger_lsn: trigger,
            })),
            Authority::RuntimeFact,
        );
        push(
            &mut frames,
            EventKind::AttentionDecided,
            EventPayload::AttentionDecided(Box::new(AttentionDecided {
                intention_id: id.clone(),
                wake_id: wake,
                decision,
                reason: format!("reason-{i}"),
            })),
            Authority::RuntimeFact,
        );
        if i == 8 {
            push(
                &mut frames,
                EventKind::IntentionCancelled,
                EventPayload::IntentionCancelled(Box::new(IntentionCancelled {
                    intention_id: id,
                    reason: "no longer needed".to_owned(),
                })),
                Authority::UserAsserted,
            );
        }
    }
    frames
}

fn bundle(store: &ProjectionStore, query: &str, now: i64) -> hm_compose::bundle::ActivationBundle {
    let counter = TokenCounter::for_model("fallback", None, FallbackWeights::default()).unwrap();
    activate_with_context(
        &store.begin_snapshot().unwrap(),
        &ActivationRequest {
            actor: ActorId::new(7),
            conversation: CONVERSATION,
            query: query.to_owned(),
            turn_text: "new turn".to_owned(),
            budget_tokens: 16_000,
            token_counter: &counter,
            maximum_candidates: 64,
            maximum_conversation_records: 64,
        },
        &ActivationContext {
            now_ns: Some(UtcNanos::new(now)),
            ..ActivationContext::default()
        },
    )
    .unwrap()
}

#[test]
fn prospective_filters_decisions_and_persists_one_quiet_hours_digest() {
    let temporary = tempfile::tempdir().unwrap();
    let frames = attention_frames();
    let store = ProjectionStore::open(temporary.path(), 32 * 1024 * 1024).unwrap();
    rebuild_projection_stream(&store, &frames, true, usize::MAX).unwrap();
    let first = bundle(&store, "", 100);
    let items = &first.sections[Tier::Prospective as usize].items;
    assert_eq!(items.len(), 4);
    for i in [0, 1, 2, 8] {
        assert!(items.iter().all(|item| {
            !String::from_utf8_lossy(&item.content).contains(&format!("objective-{i}"))
        }));
    }
    let digest = items
        .iter()
        .find(|item| item.content.starts_with(b"BATCH DIGEST"))
        .unwrap();
    assert!(String::from_utf8_lossy(&digest.content).contains("objective-6 — reason-6"));
    assert!(String::from_utf8_lossy(&digest.content).contains("objective-7 — reason-7"));
    for item in items {
        assert_eq!(item.why, WhyCode::Prospective);
        assert!(
            item.provenance
                .iter()
                .all(|lsn| first.manifest.included.contains(lsn))
        );
    }
    drop(store);
    let reopened = ProjectionStore::open(temporary.path(), 32 * 1024 * 1024).unwrap();
    assert_eq!(first, bundle(&reopened, "", 100));
    assert!(
        bundle(&reopened, "", 10_000).sections[Tier::Prospective as usize]
            .items
            .is_empty()
    );
    let rendered = safety::render(&first, digest.provenance.clone()).unwrap();
    assert!(
        rendered.sections[Tier::Prospective as usize]
            .items
            .is_empty()
    );
}

#[test]
fn procedures_are_observations_until_user_adoption_and_keep_safe_roles() {
    let mut frames = attention_frames();
    for (id, support_count) in [("tentative", 1), ("supported", 3), ("adopted", 3)] {
        let supports = (0..support_count)
            .map(|i| ProcedureSupport {
                source_root: vec![i + 1],
                conversation: vec![i % 2 + 1; 16],
                episode_lsn: u64::from(i) + 1,
            })
            .collect();
        let mined_lsn = push(
            &mut frames,
            EventKind::ProcedureMined,
            EventPayload::ProcedureMined(Box::new(ProcedureMined {
                procedure_id: id.as_bytes().to_vec(),
                strategy: "test before deploy".to_owned(),
                expected_outcomes: vec!["safe deployment".to_owned()],
                preconditions: vec!["clean repository".to_owned()],
                supports,
                failures: None,
                counterexamples: None,
            })),
            Authority::DerivedInference,
        );
        if id == "adopted" {
            push(
                &mut frames,
                EventKind::ProcedureAdopted,
                EventPayload::ProcedureAdopted(Box::new(ProcedureAdopted {
                    procedure_id: id.as_bytes().to_vec(),
                    procedure_lsn: mined_lsn,
                })),
                Authority::UserAsserted,
            );
        }
    }
    let temporary = tempfile::tempdir().unwrap();
    let store = ProjectionStore::open(temporary.path(), 32 * 1024 * 1024).unwrap();
    rebuild_projection_stream(&store, &frames, true, usize::MAX).unwrap();
    let first = bundle(&store, "how should deployment proceed", 100);
    let items = first.sections[Tier::Fused as usize]
        .items
        .iter()
        .filter(|item| item.why == WhyCode::Procedure)
        .collect::<Vec<_>>();
    assert_eq!(items.len(), 3);
    assert_eq!(
        items
            .iter()
            .filter(|item| String::from_utf8_lossy(&item.content).contains("not an instruction"))
            .count(),
        2
    );
    assert_eq!(
        items
            .iter()
            .filter(|item| item.content.starts_with(b"ADOPTED PROCEDURE"))
            .count(),
        1
    );
    let rendered = safety::render(&first, []).unwrap();
    assert!(
        rendered
            .sections
            .iter()
            .flat_map(|section| &section.items)
            .all(|item| item.role == "user")
    );
    assert!(
        bundle(&store, "deployment status", 100).sections[Tier::Fused as usize]
            .items
            .iter()
            .all(|item| item.why != WhyCode::Procedure)
    );
}
