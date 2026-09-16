#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::missing_errors_doc
)]

use crate::fsrs::{FsrsData, retrievability, reviewed_event};
use hm_schema::events::{MemoryFadeReason, MemoryFaded, Retention, ReviewRating, Reviewed};
use std::collections::{BTreeMap, BTreeSet};

const NANOS_PER_DAY: i64 = 86_400_000_000_000;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AttestationSignal {
    Used,
    Ignored,
    Helpful,
    Harmful,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReviewCandidate {
    pub memory_id: Vec<u8>,
    pub retention: Retention,
    pub provenance_lsns: BTreeSet<u64>,
    pub fsrs: FsrsData,
    pub source_lsn: u64,
    pub signals: Vec<AttestationSignal>,
    pub retrieval_rank: Option<u32>,
    pub contradiction_involved: bool,
    pub dream_access_count: u32,
    pub faded: bool,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ProtectionSet {
    lsns: BTreeSet<u64>,
}

impl ProtectionSet {
    #[must_use]
    pub fn from_sources(
        open_loops: impl IntoIterator<Item = u64>,
        active_bindings: impl IntoIterator<Item = u64>,
        unfulfilled_intentions: impl IntoIterator<Item = u64>,
        resident_beliefs: impl IntoIterator<Item = u64>,
        recent_bundles: &[Vec<u64>],
        keep_last_bundles: usize,
    ) -> Self {
        let mut lsns = open_loops.into_iter().collect::<BTreeSet<_>>();
        lsns.extend(active_bindings);
        lsns.extend(unfulfilled_intentions);
        lsns.extend(resident_beliefs);
        for bundle in recent_bundles.iter().rev().take(keep_last_bundles) {
            lsns.extend(bundle.iter().copied());
        }
        Self { lsns }
    }

    #[must_use]
    pub fn protects(&self, provenance: &BTreeSet<u64>) -> bool {
        provenance.iter().any(|lsn| self.lsns.contains(lsn))
    }

    pub fn extend(&mut self, values: impl IntoIterator<Item = u64>) {
        self.lsns.extend(values);
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.lsns.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.lsns.is_empty()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReviewDecision {
    pub score: u64,
    pub event: Reviewed,
}

#[must_use]
pub fn schedule_reviews(
    candidates: &[ReviewCandidate],
    protection: &ProtectionSet,
    reviewed_at_ns: i64,
) -> Vec<ReviewDecision> {
    let mut output = candidates
        .iter()
        .filter(|candidate| {
            !candidate.faded
                && candidate.retention != Retention::DoNotStore
                && !protection.protects(&candidate.provenance_lsns)
        })
        .map(|candidate| {
            let rating = rating_for(candidate);
            let (event, _) = reviewed_event(
                candidate.memory_id.clone(),
                candidate.source_lsn,
                reviewed_at_ns,
                candidate.fsrs,
                rating,
            );
            ReviewDecision {
                score: review_score(candidate),
                event,
            }
        })
        .collect::<Vec<_>>();
    output.sort_by(|left, right| {
        right
            .score
            .cmp(&left.score)
            .then_with(|| left.event.memory_id.cmp(&right.event.memory_id))
    });
    output
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FadePolicy {
    pub maximum_retrievability_micros: u32,
}

#[must_use]
pub fn fading_event(
    candidate: &ReviewCandidate,
    protection: &ProtectionSet,
    now_ns: i64,
    policy: FadePolicy,
) -> Option<MemoryFaded> {
    if candidate.faded
        || matches!(
            candidate.retention,
            Retention::Durable | Retention::DoNotStore
        )
        || protection.protects(&candidate.provenance_lsns)
    {
        return None;
    }
    let last = candidate.fsrs.last_review_ns?;
    let elapsed_days = now_ns.saturating_sub(last).max(0) as f64 / NANOS_PER_DAY as f64;
    let retrievability_micros =
        (retrievability(candidate.fsrs.stability, elapsed_days) * 1_000_000.0).round() as u32;
    let daily_due =
        candidate.retention != Retention::Daily || now_ns.saturating_sub(last) >= NANOS_PER_DAY;
    (daily_due && retrievability_micros <= policy.maximum_retrievability_micros).then(|| {
        MemoryFaded {
            memory_id: candidate.memory_id.clone(),
            reason: MemoryFadeReason::LowRetrievability,
            evidence_lsns: Some(candidate.provenance_lsns.iter().copied().collect()),
        }
    })
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContradictedOutcome {
    pub domain: String,
    pub observation_lsn: u64,
    pub observed_at_ns: i64,
}

#[must_use]
pub fn counterexample_protection(
    outcomes: &[ContradictedOutcome],
    keep_per_domain: usize,
) -> BTreeSet<u64> {
    let mut domains = BTreeMap::<&str, Vec<&ContradictedOutcome>>::new();
    for outcome in outcomes {
        domains.entry(&outcome.domain).or_default().push(outcome);
    }
    let mut protected = BTreeSet::new();
    for values in domains.values_mut() {
        values.sort_by(|left, right| {
            right
                .observed_at_ns
                .cmp(&left.observed_at_ns)
                .then_with(|| right.observation_lsn.cmp(&left.observation_lsn))
        });
        protected.extend(
            values
                .iter()
                .take(keep_per_domain)
                .map(|value| value.observation_lsn),
        );
    }
    protected
}

fn rating_for(candidate: &ReviewCandidate) -> ReviewRating {
    if candidate.contradiction_involved || candidate.signals.contains(&AttestationSignal::Harmful) {
        ReviewRating::Again
    } else if candidate.signals.contains(&AttestationSignal::Helpful)
        || candidate.retrieval_rank.is_some_and(|rank| rank <= 3)
    {
        ReviewRating::Easy
    } else if candidate.signals.contains(&AttestationSignal::Used) {
        ReviewRating::Good
    } else {
        ReviewRating::Hard
    }
}

fn review_score(candidate: &ReviewCandidate) -> u64 {
    let contradiction = u64::from(candidate.contradiction_involved) * 4_000_000;
    let attestations = candidate.signals.iter().fold(0_u64, |score, signal| {
        score.saturating_add(match signal {
            AttestationSignal::Harmful => 3_000_000,
            AttestationSignal::Helpful => 2_000_000,
            AttestationSignal::Used => 1_000_000,
            AttestationSignal::Ignored => 0,
        })
    });
    let rank = candidate
        .retrieval_rank
        .map_or(0, |rank| 1_000_000_u64 / u64::from(rank.max(1)));
    contradiction
        .saturating_add(attestations)
        .saturating_add(rank)
}
