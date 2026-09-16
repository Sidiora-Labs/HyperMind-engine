#![allow(clippy::missing_errors_doc)]

use std::collections::BTreeSet;

use hm_core::{Error, ErrorCode};

pub const Q16_ONE: u32 = 1 << 16;
pub const DEFAULT_SIMILARITY_THRESHOLD_Q16: u32 = 52_429;
pub const MAXIMUM_NAME_CHARS: usize = 128;
pub const MAXIMUM_CANDIDATES: usize = 4096;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AliasSuggestion {
    pub candidate: String,
    pub similarity_q16: u32,
}

#[must_use]
pub fn normalize_name(value: &str) -> String {
    let mut normalized = String::with_capacity(value.len());
    for character in value.to_lowercase().chars() {
        if character.is_alphanumeric() {
            normalized.push(character);
        } else if !normalized.ends_with('_') {
            normalized.push('_');
        }
    }
    normalized.trim_matches('_').to_owned()
}

#[must_use]
pub fn normalized_segments(value: &str, maximum: usize) -> Vec<String> {
    let mut segments: Vec<String> = normalize_name(value)
        .split('_')
        .filter(|segment| !segment.is_empty())
        .map(str::to_owned)
        .collect();
    segments.sort_unstable();
    segments.dedup();
    segments.truncate(maximum);
    segments
}

#[must_use]
pub fn similarity_q16(left: &str, right: &str) -> u32 {
    let left_name = normalize_name(left);
    let right_name = normalize_name(right);
    if left_name.is_empty()
        || right_name.is_empty()
        || exceeds_maximum_chars(&left_name)
        || exceeds_maximum_chars(&right_name)
    {
        return 0;
    }
    if left_name == right_name {
        return Q16_ONE;
    }
    let left_pairs = character_pairs(&left_name);
    let right_pairs = character_pairs(&right_name);
    let Ok(shared) = u64::try_from(left_pairs.intersection(&right_pairs).count()) else {
        return 0;
    };
    let Ok(total) = u64::try_from(left_pairs.len() + right_pairs.len()) else {
        return 0;
    };
    if total == 0 {
        return 0;
    }
    let score = 2 * shared * u64::from(Q16_ONE) / total;
    u32::try_from(score).unwrap_or(Q16_ONE)
}

pub fn suggest_aliases(
    name: &str,
    candidates: &[&str],
    threshold_q16: u32,
    limit: usize,
) -> Result<Vec<AliasSuggestion>, Error> {
    if limit == 0
        || threshold_q16 == 0
        || threshold_q16 > Q16_ONE
        || candidates.len() > MAXIMUM_CANDIDATES
    {
        return Err(Error::new(ErrorCode::InvalidArgument));
    }
    let mut suggestions: Vec<AliasSuggestion> = candidates
        .iter()
        .filter_map(|candidate| {
            let similarity = similarity_q16(name, candidate);
            (similarity >= threshold_q16).then(|| AliasSuggestion {
                candidate: (*candidate).to_owned(),
                similarity_q16: similarity,
            })
        })
        .collect();
    suggestions.sort_by(|left, right| {
        right
            .similarity_q16
            .cmp(&left.similarity_q16)
            .then_with(|| left.candidate.cmp(&right.candidate))
    });
    suggestions.truncate(limit);
    Ok(suggestions)
}

fn exceeds_maximum_chars(normalized: &str) -> bool {
    normalized.chars().take(MAXIMUM_NAME_CHARS + 1).count() > MAXIMUM_NAME_CHARS
}

fn character_pairs(normalized: &str) -> BTreeSet<(char, char)> {
    let characters: Vec<char> = normalized.chars().collect();
    characters
        .windows(2)
        .map(|pair| (pair[0], pair[1]))
        .collect()
}
