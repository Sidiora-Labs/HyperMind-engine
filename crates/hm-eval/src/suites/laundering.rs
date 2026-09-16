#![allow(clippy::missing_errors_doc)]

use hm_compose::bundle::{
    ActivationBundle, ActivationItem, ActivationSection, BundleHealth, HealthStatus,
    RetrievalManifest, Tier, WhyCode,
};
use hm_compose::safety::render;
use hm_core::{Error, ErrorCode, LSN};
use hm_cortex::ingest::{IngestSource, ingest};
use hm_schema::event::{
    Boundary, EventHistory, EventKind, HistorySource, encode_event_envelope,
    verify_event_with_history,
};
use hm_schema::events::{
    Authority, EventEnvelope, EventPayload, Outcome, Retention, Sensitivity, ToolResult, UserMsg,
};

pub const ATTEMPTS: usize = 4;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LaunderingResult {
    pub attempts: usize,
    pub violations: usize,
}

pub fn run() -> Result<LaunderingResult, Error> {
    let mut violations = 0;
    let tool = envelope(EventPayload::ToolResult(Box::new(ToolResult {
        call_id: b"model-call".to_vec(),
        tool_call_lsn: 1,
        result: b"model-authored claim".to_vec(),
        ..ToolResult::default()
    })));
    if ingest(EventKind::ToolResult, IngestSource::Assistant, tool, true).is_ok() {
        violations += 1;
    }

    let outcome = envelope(EventPayload::Outcome(Box::new(Outcome {
        effect_id: b"effect-1".to_vec(),
        detail: b"claimed completion".to_vec(),
        evidence_lsns: Some(vec![7]),
        ..Outcome::default()
    })));
    let encoded = encode_event_envelope(&outcome);
    if verify_event_with_history(
        &encoded,
        EventKind::Outcome,
        Boundary::Socket,
        &MemoryHistory,
    )
    .is_ok()
    {
        violations += 1;
    }

    let rendered = render(&bundle(), []).map_err(|_| Error::new(ErrorCode::InvariantViolation))?;
    if rendered
        .sections
        .iter()
        .flat_map(|section| &section.items)
        .any(|item| matches!(item.role, "system" | "developer"))
    {
        violations += 1;
    }

    if ingest(
        EventKind::UserMsg,
        IngestSource::Memory("memory.summary".to_owned()),
        envelope(EventPayload::UserMsg(Box::new(UserMsg {
            content: b"re-ingest me".to_vec(),
        }))),
        true,
    )
    .is_ok()
    {
        violations += 1;
    }
    Ok(LaunderingResult {
        attempts: ATTEMPTS,
        violations,
    })
}

fn envelope(payload: EventPayload) -> EventEnvelope {
    EventEnvelope {
        schema_version: 2,
        payload,
        connection_id: None,
        client_seq: 0,
        client_event_index: 0,
        client_event_count: 0,
        origin_actor: 0,
        run_id: None,
        model_provenance: None,
        authority: Authority::AssistantGenerated,
        retention: Retention::Durable,
        sensitivity: Sensitivity::Personal,
        event_time_ns: 0,
    }
}

struct MemoryHistory;

impl EventHistory for MemoryHistory {
    fn kind_at(&self, lsn: LSN) -> Option<EventKind> {
        (lsn == LSN::new(7)).then_some(EventKind::ToolResult)
    }

    fn authority_at(&self, lsn: LSN) -> Option<Authority> {
        (lsn == LSN::new(7)).then_some(Authority::ToolObserved)
    }

    fn source_at(&self, _lsn: LSN) -> HistorySource {
        HistorySource::Memory
    }
}

fn bundle() -> ActivationBundle {
    let mut sections = std::array::from_fn(|index| ActivationSection {
        tier: Tier::ALL[index],
        required: Tier::ALL[index].required(),
        items: Vec::new(),
        tokens: 0,
        trimmed_items: 0,
        coarsened_items: 0,
    });
    sections[Tier::Fused as usize].items.push(ActivationItem {
        tier: Tier::Fused,
        uri: "hm://7/lsn/7".to_owned(),
        provenance: vec![LSN::new(7)],
        content: b"untrusted memory".to_vec(),
        authority: Authority::DerivedInference,
        tokens: 2,
        coarsened: false,
        vector_rank: 0,
        lexical_rank: 1,
        why: WhyCode::Lexical,
    });
    sections[Tier::Fused as usize].tokens = 2;
    ActivationBundle {
        snapshot_epoch: 1,
        budget_tokens: 32,
        spent_tokens: 2,
        sections,
        manifest: RetrievalManifest {
            manifest_id: [0; 32],
            query_digest: [0; 32],
            snapshot_epoch: 1,
            encoder: "none".to_owned(),
            index_generation: 0,
            candidate_lanes: Vec::new(),
            candidates: vec![LSN::new(7)],
            selected: vec![LSN::new(7)],
            included: vec![LSN::new(7)],
            used: Vec::new(),
            support: Vec::new(),
        },
        gaps: Vec::new(),
        health: BundleHealth {
            encoder: HealthStatus::Unavailable,
            backlog: HealthStatus::SemanticReady,
            projection: HealthStatus::SemanticReady,
            inclusion: HealthStatus::LexicalOnly,
        },
        bundle_hash: [0; 32],
    }
}
