use core::ops::Range;
use std::collections::HashMap;

use crate::chunk::paragraph_units;
use crate::identity::chunk_content_hash;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChangedRegion {
    pub old: Range<usize>,
    pub new: Range<usize>,
}

struct Units {
    spans: Vec<Range<usize>>,
    hashes: Vec<[u8; 32]>,
}

struct Anchor {
    old: usize,
    new: usize,
}

#[must_use]
pub fn changed_regions(old_text: &str, new_text: &str) -> Vec<ChangedRegion> {
    if old_text == new_text {
        return Vec::new();
    }
    let old = units_of(old_text);
    let new = units_of(new_text);

    let mut prefix = 0;
    while prefix < old.hashes.len()
        && prefix < new.hashes.len()
        && old.hashes[prefix] == new.hashes[prefix]
    {
        prefix += 1;
    }
    let mut old_end = old.hashes.len();
    let mut new_end = new.hashes.len();
    while old_end > prefix && new_end > prefix && old.hashes[old_end - 1] == new.hashes[new_end - 1]
    {
        old_end -= 1;
        new_end -= 1;
    }

    let old_middle = middle_bounds(&old, prefix, old_end, old_text.len());
    let new_middle = middle_bounds(&new, prefix, new_end, new_text.len());

    let anchors = anchors_between(&old, prefix..old_end, &new, prefix..new_end);

    let mut regions = Vec::new();
    let mut old_cursor = old_middle.start;
    let mut new_cursor = new_middle.start;
    for anchor in &anchors {
        let old_span = old.spans[anchor.old].clone();
        let new_span = new.spans[anchor.new].clone();
        push_region(
            &mut regions,
            old_text,
            new_text,
            old_cursor..old_span.start,
            new_cursor..new_span.start,
        );
        old_cursor = old_span.end;
        new_cursor = new_span.end;
    }
    push_region(
        &mut regions,
        old_text,
        new_text,
        old_cursor..old_middle.end,
        new_cursor..new_middle.end,
    );
    regions
}

fn units_of(text: &str) -> Units {
    let spans = paragraph_units(text);
    let hashes = spans
        .iter()
        .map(|span| chunk_content_hash(&text[span.clone()]))
        .collect();
    Units { spans, hashes }
}

fn middle_bounds(units: &Units, prefix: usize, end: usize, length: usize) -> Range<usize> {
    let start = if prefix == 0 {
        0
    } else {
        units.spans[prefix - 1].end
    };
    let finish = if end == units.spans.len() {
        length
    } else {
        units.spans[end].start
    };
    start..finish.max(start)
}

fn anchors_between(
    old: &Units,
    old_range: Range<usize>,
    new: &Units,
    new_range: Range<usize>,
) -> Vec<Anchor> {
    let old_unique = unique_positions(&old.hashes[old_range.clone()], old_range.start);
    let new_unique = unique_positions(&new.hashes[new_range.clone()], new_range.start);
    let mut candidates = Vec::new();
    for position in old_range {
        let Some(old_position) = old_unique.get(&old.hashes[position]) else {
            continue;
        };
        if *old_position != position {
            continue;
        }
        let Some(new_position) = new_unique.get(&old.hashes[position]) else {
            continue;
        };
        candidates.push(Anchor {
            old: position,
            new: *new_position,
        });
    }
    increasing_run(candidates)
}

fn unique_positions(hashes: &[[u8; 32]], offset: usize) -> HashMap<[u8; 32], usize> {
    let mut counts: HashMap<[u8; 32], (usize, usize)> = HashMap::new();
    for (index, hash) in hashes.iter().enumerate() {
        let entry = counts.entry(*hash).or_insert((0, offset + index));
        entry.0 += 1;
    }
    counts
        .into_iter()
        .filter_map(|(hash, (count, position))| (count == 1).then_some((hash, position)))
        .collect()
}

fn increasing_run(candidates: Vec<Anchor>) -> Vec<Anchor> {
    if candidates.is_empty() {
        return candidates;
    }
    let mut tails: Vec<usize> = Vec::new();
    let mut previous: Vec<Option<usize>> = Vec::with_capacity(candidates.len());
    for (index, candidate) in candidates.iter().enumerate() {
        let position = tails.partition_point(|tail| candidates[*tail].new < candidate.new);
        previous.push(if position == 0 {
            None
        } else {
            Some(tails[position - 1])
        });
        if position == tails.len() {
            tails.push(index);
        } else {
            tails[position] = index;
        }
    }
    let mut chosen = Vec::with_capacity(tails.len());
    let mut cursor = tails.last().copied();
    while let Some(index) = cursor {
        chosen.push(index);
        cursor = previous[index];
    }
    chosen.reverse();
    let mut kept = Vec::with_capacity(chosen.len());
    for (index, candidate) in candidates.into_iter().enumerate() {
        if chosen.binary_search(&index).is_ok() {
            kept.push(candidate);
        }
    }
    kept
}

fn push_region(
    regions: &mut Vec<ChangedRegion>,
    old_text: &str,
    new_text: &str,
    old: Range<usize>,
    new: Range<usize>,
) {
    if old.is_empty() && new.is_empty() {
        return;
    }
    let shaved = shave(old_text, new_text, old, new);
    if shaved.old.is_empty() && shaved.new.is_empty() {
        return;
    }
    regions.push(shaved);
}

fn shave(old_text: &str, new_text: &str, old: Range<usize>, new: Range<usize>) -> ChangedRegion {
    let old_bytes = &old_text.as_bytes()[old.clone()];
    let new_bytes = &new_text.as_bytes()[new.clone()];
    let limit = old_bytes.len().min(new_bytes.len());

    let mut head = 0;
    while head < limit && old_bytes[head] == new_bytes[head] {
        head += 1;
    }
    while head > 0
        && (!old_text.is_char_boundary(old.start + head)
            || !new_text.is_char_boundary(new.start + head))
    {
        head -= 1;
    }

    let mut tail = 0;
    let remaining = limit - head;
    while tail < remaining
        && old_bytes[old_bytes.len() - 1 - tail] == new_bytes[new_bytes.len() - 1 - tail]
    {
        tail += 1;
    }
    while tail > 0
        && (!old_text.is_char_boundary(old.end - tail)
            || !new_text.is_char_boundary(new.end - tail))
    {
        tail -= 1;
    }

    ChangedRegion {
        old: (old.start + head)..(old.end - tail),
        new: (new.start + head)..(new.end - tail),
    }
}
