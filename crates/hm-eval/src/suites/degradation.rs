#![allow(clippy::missing_errors_doc)]

use hm_compose::budget::BudgetProfile;
use hm_compose::bundle::{
    ActivationContext, ActivationRequest, GapKind, Tier, activate, activate_with_context,
};
use hm_compose::tokens::{FallbackWeights, TokenCounter};
use hm_core::{ActorId, ConversationId, Error, ErrorCode, LSN, UtcNanos};
use hm_ledger::frame::{EventKind, Frame, FrameHeader};
use hm_proj::bindings::BindingRequirement;
use hm_proj::rebuild::rebuild_projection_stream;
use hm_proj::store::ProjectionStore;
use hm_schema::event::encode_event_envelope;
use hm_schema::events::{
    Authority, Binding, EventEnvelope, EventPayload, IntentSet, LoopOpened, Retention, Sensitivity,
    UserMsg,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DegradationResult {
    pub forced_conditions: usize,
    pub visible_conditions: usize,
}

pub fn run() -> Result<DegradationResult, Error> {
    let temporary = tempfile::tempdir().map_err(|_| Error::new(ErrorCode::OpenFailed))?;
    let store = ProjectionStore::open(temporary.path(), 32 * 1024 * 1024)?;
    let conversation = ConversationId::derive("slice2-degradation");
    let frames = workload(conversation);
    rebuild_projection_stream(&store, &frames, true, usize::MAX)?;
    let snapshot = store.begin_snapshot()?;
    let counter = TokenCounter::for_model("fallback", None, FallbackWeights::default())?;
    let request = |budget_tokens| ActivationRequest {
        actor: ActorId::new(7),
        conversation,
        query: "optional continuity context".to_owned(),
        turn_text: "resume".to_owned(),
        budget_tokens,
        token_counter: &counter,
        maximum_candidates: 64,
        maximum_conversation_records: 64,
    };

    let missing = activate_with_context(
        &snapshot,
        &request(16_384),
        &ActivationContext {
            task: Some(b"active-task".to_vec()),
            required_bindings: vec![BindingRequirement {
                canonical_entity: "missing-service".to_owned(),
                property: "revision".to_owned(),
                revision: Some(b"m1".to_vec()),
                freshness_requirement_ns: Some(100),
            }],
            now_ns: Some(UtcNanos::new(2_000)),
            budget_profile: BudgetProfile::default(),
        },
    )?;
    require_gap(&missing.gaps, GapKind::MissingBinding)?;

    let full = activate(&snapshot, &request(16_384))?;
    let required_tokens = full.sections[..=Tier::WorkLedger as usize]
        .iter()
        .try_fold(0_usize, |total, section| {
            total
                .checked_add(section.tokens)
                .ok_or_else(|| Error::new(ErrorCode::CapacityExceeded))
        })?;
    let dropped = activate(&snapshot, &request(required_tokens.max(1)))?;
    require_gap(&dropped.gaps, GapKind::DroppedTier)?;

    let narrowed = (1..required_tokens)
        .find_map(|budget| {
            activate(&snapshot, &request(budget)).ok().filter(|bundle| {
                bundle
                    .gaps
                    .iter()
                    .any(|gap| gap.kind == GapKind::NarrowedSubtask)
            })
        })
        .ok_or_else(|| Error::new(ErrorCode::InvariantViolation))?;
    require_gap(&narrowed.gaps, GapKind::NarrowedSubtask)?;
    Ok(DegradationResult {
        forced_conditions: 3,
        visible_conditions: 3,
    })
}

fn require_gap(gaps: &[hm_compose::bundle::Gap], expected: GapKind) -> Result<(), Error> {
    if gaps.iter().any(|gap| gap.kind == expected) {
        Ok(())
    } else {
        Err(Error::new(ErrorCode::InvariantViolation))
    }
}

fn workload(conversation: ConversationId) -> Vec<Frame> {
    vec![
        frame(
            1,
            conversation,
            EventKind::UserMsg,
            EventPayload::UserMsg(Box::new(UserMsg {
                content: b"optional continuity context that can be dropped under pressure".to_vec(),
            })),
        ),
        frame(
            2,
            conversation,
            EventKind::IntentSet,
            EventPayload::IntentSet(Box::new(IntentSet {
                objective: b"make all degraded conditions explicit".to_vec(),
            })),
        ),
        frame(
            3,
            conversation,
            EventKind::LoopOpened,
            EventPayload::LoopOpened(Box::new(LoopOpened {
                loop_id: b"older-task".to_vec(),
                objective: b"retain an older task until budget pressure".to_vec(),
            })),
        ),
        frame(
            4,
            conversation,
            EventKind::LoopOpened,
            EventPayload::LoopOpened(Box::new(LoopOpened {
                loop_id: b"active-task".to_vec(),
                objective: b"continue the exact active task".to_vec(),
            })),
        ),
        frame(
            5,
            conversation,
            EventKind::Binding,
            EventPayload::Binding(Box::new(Binding {
                task: Some(b"active-task".to_vec()),
                scope: None,
                canonical_entity: "repository".to_owned(),
                property: "revision".to_owned(),
                evidence_lsn: 1,
                revision: b"r1".to_vec(),
                freshness_requirement_ns: 10_000,
            })),
        ),
    ]
}

fn frame(lsn: u64, conversation: ConversationId, kind: EventKind, payload: EventPayload) -> Frame {
    Frame {
        header: FrameHeader {
            lsn: LSN::new(lsn),
            kind,
            wall_timestamp_ns: UtcNanos::new(i64::try_from(lsn).unwrap_or_default()),
            actor: ActorId::new(7),
            conversation,
        },
        sealed_payload: encode_event_envelope(&EventEnvelope {
            schema_version: 2,
            payload,
            connection_id: None,
            client_seq: 0,
            client_event_index: 0,
            client_event_count: 0,
            origin_actor: 7,
            run_id: None,
            model_provenance: None,
            authority: Authority::UserAsserted,
            retention: Retention::CurrentState,
            sensitivity: Sensitivity::Personal,
            event_time_ns: i64::try_from(lsn).unwrap_or_default(),
        }),
    }
}
