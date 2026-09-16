#![allow(clippy::missing_errors_doc)]

use hm_core::{Error, ErrorCode};
use hm_schema::events::{AttentionDecided, AttentionDecision};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AttentionFactors {
    pub urgency: u32,
    pub expected_value: u32,
    pub confidence: u32,
    pub interruption_cost: u32,
    pub resource_cost: u32,
    pub duplication_penalty: u32,
    pub quiet_hours: bool,
    pub notifications_remaining: u32,
    pub workload: u32,
}

pub fn decide(
    intention_id: &[u8],
    wake_id: &[u8],
    factors: AttentionFactors,
) -> Result<AttentionDecided, Error> {
    if intention_id.is_empty()
        || wake_id.is_empty()
        || [
            factors.urgency,
            factors.expected_value,
            factors.confidence,
            factors.interruption_cost,
            factors.resource_cost,
            factors.duplication_penalty,
            factors.workload,
        ]
        .into_iter()
        .any(|value| value > 1_000_000)
    {
        return Err(Error::new(ErrorCode::InvalidArgument));
    }
    let benefit = u64::from(factors.urgency)
        .saturating_add(u64::from(factors.expected_value))
        .saturating_add(u64::from(factors.confidence));
    let cost = u64::from(factors.interruption_cost)
        .saturating_add(u64::from(factors.resource_cost))
        .saturating_add(u64::from(factors.duplication_penalty));
    let score = benefit.saturating_sub(cost);
    let (decision, reason) = if factors.duplication_penalty >= 900_000 {
        (AttentionDecision::Ignore, "duplicate wake suppressed")
    } else if factors.quiet_hours {
        (
            AttentionDecision::Batch,
            "quiet hours require digest batching",
        )
    } else if factors.workload >= 900_000 {
        (
            AttentionDecision::Schedule,
            "current workload requires re-arming",
        )
    } else if factors.notifications_remaining == 0 {
        (AttentionDecision::Remember, "notification budget exhausted")
    } else if score >= 1_800_000 {
        (AttentionDecision::Notify, "high-value urgent wake")
    } else if score >= 1_000_000 {
        (
            AttentionDecision::StartWork,
            "actionable wake within budget",
        )
    } else if score >= 500_000 {
        (AttentionDecision::AskUser, "decision requires user input")
    } else {
        (
            AttentionDecision::Remember,
            "wake retained without interruption",
        )
    };
    Ok(AttentionDecided {
        intention_id: intention_id.to_vec(),
        wake_id: wake_id.to_vec(),
        decision,
        reason: format!("{reason}; score={score}"),
    })
}
