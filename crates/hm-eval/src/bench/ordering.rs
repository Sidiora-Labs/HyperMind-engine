#![allow(clippy::cast_precision_loss, clippy::missing_errors_doc)]

//! Judge-free scoring for probes that ask for events in order.
//!
//! No language model takes part: observed events are aligned to the reference
//! events by term overlap, so the alignment rewards lexical similarity to the
//! reference wording rather than meaning. `OrderingScore` reports `matched`,
//! `precision` and `recall` beside the order term so a reader can see how much
//! of a score came from covering the events and how much from ordering them.

use super::gateway::DynError;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::BTreeSet;

pub const DEFAULT_MINIMUM_OVERLAP: f64 = 0.34;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Alignment {
    pub observed_to_reference: Vec<Option<usize>>,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct OrderingScore {
    pub reference_events: usize,
    pub observed_events: usize,
    pub matched: usize,
    pub precision: f64,
    pub recall: f64,
    pub coverage_f1: f64,
    pub rank_correlation: f64,
    pub normalized_correlation: f64,
    pub score: f64,
}

#[must_use]
pub fn rank_correlation(left: &[usize], right: &[usize]) -> f64 {
    if left.len() != right.len() || left.len() < 2 {
        return 0.0;
    }
    let mut agreement: i64 = 0;
    let mut pairs: i64 = 0;
    let mut left_ties: i64 = 0;
    let mut right_ties: i64 = 0;
    for (position, (left_first, right_first)) in left.iter().zip(right.iter()).enumerate() {
        for (left_second, right_second) in left.iter().zip(right.iter()).skip(position + 1) {
            pairs += 1;
            let left_order = left_first.cmp(left_second);
            let right_order = right_first.cmp(right_second);
            if left_order == Ordering::Equal {
                left_ties += 1;
            }
            if right_order == Ordering::Equal {
                right_ties += 1;
            }
            if left_order != Ordering::Equal && right_order != Ordering::Equal {
                agreement += if left_order == right_order { 1 } else { -1 };
            }
        }
    }
    let comparable = ((pairs - left_ties) * (pairs - right_ties)) as f64;
    if comparable <= 0.0 {
        return 0.0;
    }
    (agreement as f64 / comparable.sqrt()).clamp(-1.0, 1.0)
}

#[must_use]
pub fn align_by_terms(
    reference: &[String],
    observed: &[String],
    minimum_overlap: f64,
) -> Alignment {
    let reference_terms: Vec<BTreeSet<String>> =
        reference.iter().map(|event| terms(event)).collect();
    let mut taken = vec![false; reference.len()];
    let mut observed_to_reference = Vec::with_capacity(observed.len());
    for event in observed {
        let event_terms = terms(event);
        let mut best: Option<(usize, f64)> = None;
        for (index, candidate) in reference_terms.iter().enumerate() {
            if taken[index] {
                continue;
            }
            let overlap = term_overlap(&event_terms, candidate);
            if overlap < minimum_overlap {
                continue;
            }
            if best.is_none_or(|(_, highest)| overlap > highest) {
                best = Some((index, overlap));
            }
        }
        if let Some((index, _)) = best {
            taken[index] = true;
            observed_to_reference.push(Some(index));
        } else {
            observed_to_reference.push(None);
        }
    }
    Alignment {
        observed_to_reference,
    }
}

pub fn score_ordering(
    reference: &[String],
    observed: &[String],
    alignment: &Alignment,
) -> Result<OrderingScore, DynError> {
    if reference.is_empty() {
        return Err("an ordering probe carries no reference events to score against".into());
    }
    if alignment.observed_to_reference.len() != observed.len() {
        return Err(format!(
            "the alignment covers {} observed events and not the {} that were observed",
            alignment.observed_to_reference.len(),
            observed.len()
        )
        .into());
    }
    let mut claimed = BTreeSet::new();
    let mut observed_ranks = Vec::with_capacity(observed.len());
    let mut reference_ranks = Vec::with_capacity(observed.len());
    let mut matched = 0;
    for (position, slot) in alignment.observed_to_reference.iter().enumerate() {
        observed_ranks.push(position);
        match slot {
            Some(index) => {
                if *index >= reference.len() {
                    return Err(format!(
                        "the alignment maps an observed event onto reference event {index}, which is absent from a reference list of {} events",
                        reference.len()
                    )
                    .into());
                }
                if !claimed.insert(*index) {
                    return Err(format!(
                        "the alignment maps two observed events onto reference event {index}"
                    )
                    .into());
                }
                matched += 1;
                reference_ranks.push(*index);
            }
            None => reference_ranks.push(reference.len()),
        }
    }
    let precision = if observed.is_empty() {
        0.0
    } else {
        matched as f64 / observed.len() as f64
    };
    let recall = matched as f64 / reference.len() as f64;
    let coverage_f1 = if precision + recall <= 0.0 {
        0.0
    } else {
        2.0 * precision * recall / (precision + recall)
    };
    let correlation = rank_correlation(&observed_ranks, &reference_ranks);
    let normalized_correlation = if observed_ranks.len() < 2 {
        1.0
    } else {
        f64::midpoint(correlation, 1.0)
    };
    Ok(OrderingScore {
        reference_events: reference.len(),
        observed_events: observed.len(),
        matched,
        precision,
        recall,
        coverage_f1,
        rank_correlation: correlation,
        normalized_correlation,
        score: normalized_correlation * coverage_f1,
    })
}

#[must_use]
pub fn split_ordered_answer(answer: &str) -> Vec<String> {
    let numbered: Vec<String> = answer.lines().filter_map(numbered_event).collect();
    if !numbered.is_empty() {
        return numbered;
    }
    let bulleted: Vec<String> = answer.lines().filter_map(bulleted_event).collect();
    if !bulleted.is_empty() {
        return bulleted;
    }
    answer
        .split(['.', '!', '?', '\n'])
        .map(str::trim)
        .filter(|sentence| !sentence.is_empty())
        .map(str::to_owned)
        .collect()
}

fn numbered_event(line: &str) -> Option<String> {
    let trimmed = line.trim();
    let digits = trimmed.len()
        - trimmed
            .trim_start_matches(|c: char| c.is_ascii_digit())
            .len();
    if digits == 0 {
        return None;
    }
    let rest = trimmed.get(digits..)?;
    let rest = rest.strip_prefix('.').or_else(|| rest.strip_prefix(')'))?;
    let event = rest.trim();
    (!event.is_empty()).then(|| event.to_owned())
}

fn bulleted_event(line: &str) -> Option<String> {
    let trimmed = line.trim();
    let rest = trimmed
        .strip_prefix("- ")
        .or_else(|| trimmed.strip_prefix("* "))
        .or_else(|| trimmed.strip_prefix("• "))?;
    let event = rest.trim();
    (!event.is_empty()).then(|| event.to_owned())
}

fn terms(event: &str) -> BTreeSet<String> {
    event
        .split_whitespace()
        .map(|word| {
            word.trim_matches(|character: char| !character.is_alphanumeric())
                .to_lowercase()
        })
        .filter(|word| !word.is_empty())
        .collect()
}

fn term_overlap(left: &BTreeSet<String>, right: &BTreeSet<String>) -> f64 {
    if left.is_empty() || right.is_empty() {
        return 0.0;
    }
    let shared = left.intersection(right).count();
    if shared == 0 {
        return 0.0;
    }
    shared as f64 / left.union(right).count() as f64
}
