#![allow(clippy::missing_errors_doc)]

use crate::bundle::{ActivationItem, Gap, GapKind, Tier, WhyCode};
use crate::tokens::TokenCounter;
use hm_core::{ActorId, ConversationId, Error, UtcNanos};
use hm_proj::bindings::{
    BindingRequirement, BindingStatus, BindingTarget, BindingsProjection, resolve_required_bindings,
};
use hm_proj::store::ReadSnapshot;
use hm_proj::timeline::read_conversation_record;

pub(crate) struct BindingsTier {
    pub items: Vec<ActivationItem>,
    pub gaps: Vec<Gap>,
}

pub(crate) fn read(
    snapshot: &ReadSnapshot<'_>,
    actor: ActorId,
    conversation: ConversationId,
    task: Option<&[u8]>,
    required: &[BindingRequirement],
    now_ns: UtcNanos,
    counter: &TokenCounter,
) -> Result<BindingsTier, Error> {
    let Some(task) = task else {
        return Ok(BindingsTier {
            items: Vec::new(),
            gaps: required
                .iter()
                .map(|requirement| gap(BindingStatus::Missing, requirement))
                .collect(),
        });
    };
    let requirements = if required.is_empty() {
        BindingsProjection::read_target(snapshot, &BindingTarget::Task(task.to_vec()))?
            .into_iter()
            .map(|record| BindingRequirement {
                canonical_entity: record.canonical_entity,
                property: record.property,
                revision: Some(record.revision),
                freshness_requirement_ns: Some(record.freshness_requirement_ns),
            })
            .collect()
    } else {
        required.to_vec()
    };
    let resolutions = resolve_required_bindings(snapshot, task, &requirements, now_ns)?;
    let mut items = Vec::new();
    let mut gaps = Vec::new();
    for resolution in resolutions {
        let mut status = resolution.status;
        if resolution.binding.as_ref().is_some_and(|record| {
            read_conversation_record(snapshot, record.evidence_lsn)
                .ok()
                .flatten()
                .is_none()
        }) {
            status = BindingStatus::Missing;
        }
        if status != BindingStatus::Resolved {
            gaps.push(gap(status, &resolution.requirement));
        }
        let Some(binding) = resolution.binding else {
            continue;
        };
        let mut provenance = vec![binding.binding_lsn, binding.evidence_lsn];
        provenance.sort_unstable();
        provenance.dedup();
        let status_name = status_name(status);
        let content = format!(
            "binding {} {} revision={} evidence_lsn={} status={status_name}",
            binding.canonical_entity,
            binding.property,
            hex(&binding.revision),
            binding.evidence_lsn
        )
        .into_bytes();
        items.push(ActivationItem {
            tier: Tier::Bindings,
            uri: format!(
                "hm://{actor}/{conversation}/{}?src=binding&task={}&entity={}&property={}&why=binding",
                binding.binding_lsn,
                hex(task),
                hex(binding.canonical_entity.as_bytes()),
                hex(binding.property.as_bytes())
            ),
            provenance,
            tokens: counter.count(&content)?,
            content,
            coarsened: false,
            vector_rank: 0,
            lexical_rank: 0,
            why: WhyCode::Binding,
        });
    }
    Ok(BindingsTier { items, gaps })
}

fn gap(status: BindingStatus, requirement: &BindingRequirement) -> Gap {
    let (kind, name) = match status {
        BindingStatus::Resolved => (GapKind::MissingBinding, "resolved"),
        BindingStatus::Missing => (GapKind::MissingBinding, "missing"),
        BindingStatus::Stale => (GapKind::StaleBinding, "stale"),
        BindingStatus::Conflicting => (GapKind::ConflictingBinding, "conflicting"),
    };
    Gap {
        kind,
        tier: Some(Tier::Bindings),
        lane: None,
        detail: format!(
            "{name} binding {} {}",
            requirement.canonical_entity, requirement.property
        ),
    }
}

const fn status_name(status: BindingStatus) -> &'static str {
    match status {
        BindingStatus::Resolved => "resolved",
        BindingStatus::Missing => "missing",
        BindingStatus::Stale => "stale",
        BindingStatus::Conflicting => "conflicting",
    }
}

fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(char::from(DIGITS[usize::from(byte >> 4)]));
        output.push(char::from(DIGITS[usize::from(byte & 0x0f)]));
    }
    output
}
