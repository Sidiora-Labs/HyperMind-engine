use hm_schema::event::EventKind;
use hm_schema::events::{Authority, Sensitivity};
use std::ops::Range;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CitedSource<'a> {
    pub authority: Authority,
    pub bytes: &'a [u8],
    pub cited_range: Range<usize>,
}

#[must_use]
pub const fn authority_for_event(kind: EventKind) -> Authority {
    match kind {
        EventKind::UserMsg => Authority::UserAsserted,
        EventKind::DeliveredMsg | EventKind::Reasoning => Authority::AssistantGenerated,
        EventKind::ToolResult => Authority::ToolObserved,
        EventKind::ProviderFrame | EventKind::MediaRef => Authority::ExternalObserved,
        EventKind::Assertion
        | EventKind::Consolidation
        | EventKind::Embedding
        | EventKind::ProposedAssertion
        | EventKind::MemoryMinted
        | EventKind::MemoryRevised
        | EventKind::MemoryMerged
        | EventKind::EdgeAsserted
        | EventKind::EdgeRetracted => Authority::DerivedInference,
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
        | EventKind::Reviewed => Authority::RuntimeFact,
    }
}

#[must_use]
pub fn derive_authority(source: &CitedSource<'_>, output: &[u8]) -> Authority {
    source
        .bytes
        .get(source.cited_range.clone())
        .filter(|quoted| *quoted == output)
        .map_or(Authority::DerivedInference, |_| source.authority)
}

#[must_use]
pub const fn cap_automatic_sensitivity(sensitivity: Sensitivity) -> Sensitivity {
    match sensitivity {
        Sensitivity::Secret => Sensitivity::Personal,
        other => other,
    }
}
