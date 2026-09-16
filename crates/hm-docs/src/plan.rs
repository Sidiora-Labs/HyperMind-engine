#![allow(clippy::missing_errors_doc)]

use std::collections::BTreeSet;

use hm_core::{Error, ErrorCode};

use crate::chunk::{ChunkCut, ChunkSpan, tiles};
use crate::diff::{ChangedRegion, changed_regions};
use crate::identity::{OccurrenceCounter, chunk_content_hash, chunk_identity};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChangeKind {
    Retained,
    Moved,
    Replaced,
    Added,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StoredChunk {
    pub chunk_id: [u8; 32],
    pub content_hash: [u8; 32],
    pub occurrence: u32,
    pub ordinal: u32,
    pub byte_start: usize,
    pub byte_end: usize,
    pub cut: ChunkCut,
    pub page_number: u32,
    pub row_index: u32,
    pub column_start: u32,
    pub column_end: u32,
    pub token_estimate: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PlannedChunk {
    pub chunk_id: [u8; 32],
    pub content_hash: [u8; 32],
    pub occurrence: u32,
    pub ordinal: u32,
    pub byte_start: usize,
    pub byte_end: usize,
    pub cut: ChunkCut,
    pub change: ChangeKind,
    pub page_number: u32,
    pub row_index: u32,
    pub column_start: u32,
    pub column_end: u32,
    pub token_estimate: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChangePlan {
    pub document_id: [u8; 32],
    pub chunks: Vec<PlannedChunk>,
    pub regions: u32,
    pub retained: u32,
    pub moved: u32,
    pub replaced: u32,
    pub added: u32,
}

pub fn plan_initial(
    document_id: [u8; 32],
    text: &str,
    spans: &[ChunkSpan],
) -> Result<ChangePlan, Error> {
    if !tiles(spans, text.len()) {
        return Err(violation());
    }
    let mut drafts = Vec::with_capacity(spans.len());
    for span in spans {
        drafts.push(span_draft(text, span, 0)?);
    }
    assemble(document_id, drafts, 0)
}

pub fn plan_revision(
    document_id: [u8; 32],
    old_text: &str,
    stored: &[StoredChunk],
    new_text: &str,
    region_chunker: &dyn Fn(&str) -> Result<Vec<ChunkSpan>, Error>,
) -> Result<ChangePlan, Error> {
    ensure_stored_tiling(stored, old_text)?;
    let runs = merged_runs(&changed_regions(old_text, new_text), stored, new_text.len());
    let mut drafts: Vec<Draft> = Vec::new();
    let mut cursor = 0;
    let mut index = 0;
    for run in &runs {
        while index < run.first {
            cursor = push_kept(&mut drafts, new_text, &stored[index], cursor)?;
            index += 1;
        }
        if index > run.first || cursor != run.new_start {
            return Err(violation());
        }
        let region = new_text
            .get(run.new_start..run.new_end)
            .ok_or_else(violation)?;
        let spans = region_chunker(region)?;
        if !tiles(&spans, region.len()) {
            return Err(violation());
        }
        let mut claimed = vec![false; run.end - run.first];
        for span in &spans {
            let mut draft = span_draft(new_text, span, run.new_start)?;
            draft.survivor = claim_replacement(stored, run, &mut claimed, &draft.content_hash);
            drafts.push(draft);
        }
        cursor = run.new_end;
        index = run.end;
    }
    while index < stored.len() {
        cursor = push_kept(&mut drafts, new_text, &stored[index], cursor)?;
        index += 1;
    }
    if cursor != new_text.len() {
        return Err(violation());
    }
    let regions = u32::try_from(runs.len()).map_err(|_| Error::new(ErrorCode::CapacityExceeded))?;
    assemble(document_id, drafts, regions)
}

pub fn validate_plan(
    plan: &ChangePlan,
    stored: &[StoredChunk],
    new_text: &str,
) -> Result<(), Error> {
    let mut positions: Vec<Option<&PlannedChunk>> = vec![None; plan.chunks.len()];
    for chunk in &plan.chunks {
        let position = usize::try_from(chunk.ordinal).map_err(|_| violation())?;
        let slot = positions.get_mut(position).ok_or_else(violation)?;
        if slot.is_some() {
            return Err(violation());
        }
        *slot = Some(chunk);
    }

    let mut identities: BTreeSet<[u8; 32]> = BTreeSet::new();
    let mut rebuilt = String::with_capacity(new_text.len());
    let mut cursor = 0;
    let mut tally = Tally::default();
    for slot in positions {
        let chunk = slot.ok_or_else(violation)?;
        if chunk.byte_start != cursor || chunk.byte_end <= chunk.byte_start {
            return Err(violation());
        }
        let text = new_text
            .get(chunk.byte_start..chunk.byte_end)
            .ok_or_else(violation)?;
        if chunk_content_hash(text) != chunk.content_hash
            || chunk_identity(&plan.document_id, &chunk.content_hash, chunk.occurrence)
                != chunk.chunk_id
            || !identities.insert(chunk.chunk_id)
        {
            return Err(violation());
        }
        ensure_lineage(chunk, stored)?;
        tally.count(chunk.change);
        rebuilt.push_str(text);
        cursor = chunk.byte_end;
    }
    if cursor != new_text.len() || rebuilt != new_text || !tally.matches(plan) {
        return Err(violation());
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, Default)]
struct Tally {
    retained: u32,
    moved: u32,
    replaced: u32,
    added: u32,
}

impl Tally {
    fn count(&mut self, change: ChangeKind) {
        let counter = match change {
            ChangeKind::Retained => &mut self.retained,
            ChangeKind::Moved => &mut self.moved,
            ChangeKind::Replaced => &mut self.replaced,
            ChangeKind::Added => &mut self.added,
        };
        *counter = counter.saturating_add(1);
    }

    fn matches(self, plan: &ChangePlan) -> bool {
        self.retained == plan.retained
            && self.moved == plan.moved
            && self.replaced == plan.replaced
            && self.added == plan.added
    }
}

#[derive(Clone, Copy, Debug)]
struct Survivor {
    chunk_id: [u8; 32],
    occurrence: u32,
    ordinal: u32,
    recut: bool,
}

struct Draft {
    byte_start: usize,
    byte_end: usize,
    cut: ChunkCut,
    page_number: u32,
    row_index: u32,
    column_start: u32,
    column_end: u32,
    token_estimate: u32,
    content_hash: [u8; 32],
    survivor: Option<Survivor>,
}

#[derive(Clone, Copy, Debug)]
struct Run {
    first: usize,
    end: usize,
    new_start: usize,
    new_end: usize,
}

fn assemble(document_id: [u8; 32], drafts: Vec<Draft>, regions: u32) -> Result<ChangePlan, Error> {
    let mut counter = OccurrenceCounter::new();
    for draft in &drafts {
        if let Some(survivor) = draft.survivor {
            counter.reserve(&draft.content_hash, survivor.occurrence);
        }
    }
    let mut chunks = Vec::with_capacity(drafts.len());
    let mut tally = Tally::default();
    for (position, draft) in drafts.into_iter().enumerate() {
        let ordinal =
            u32::try_from(position).map_err(|_| Error::new(ErrorCode::CapacityExceeded))?;
        let (chunk_id, occurrence, change) = match draft.survivor {
            Some(survivor) if survivor.recut => {
                (survivor.chunk_id, survivor.occurrence, ChangeKind::Replaced)
            }
            Some(survivor) if survivor.ordinal == ordinal => {
                (survivor.chunk_id, survivor.occurrence, ChangeKind::Retained)
            }
            Some(survivor) => (survivor.chunk_id, survivor.occurrence, ChangeKind::Moved),
            None => {
                let occurrence = counter.next(&draft.content_hash);
                let chunk_id = chunk_identity(&document_id, &draft.content_hash, occurrence);
                (chunk_id, occurrence, ChangeKind::Added)
            }
        };
        tally.count(change);
        chunks.push(PlannedChunk {
            chunk_id,
            content_hash: draft.content_hash,
            occurrence,
            ordinal,
            byte_start: draft.byte_start,
            byte_end: draft.byte_end,
            cut: draft.cut,
            change,
            page_number: draft.page_number,
            row_index: draft.row_index,
            column_start: draft.column_start,
            column_end: draft.column_end,
            token_estimate: draft.token_estimate,
        });
    }
    Ok(ChangePlan {
        document_id,
        chunks,
        regions,
        retained: tally.retained,
        moved: tally.moved,
        replaced: tally.replaced,
        added: tally.added,
    })
}

fn span_draft(text: &str, span: &ChunkSpan, offset: usize) -> Result<Draft, Error> {
    let byte_start = span.byte_start.checked_add(offset).ok_or_else(violation)?;
    let byte_end = span.byte_end.checked_add(offset).ok_or_else(violation)?;
    let slice = text.get(byte_start..byte_end).ok_or_else(violation)?;
    Ok(Draft {
        byte_start,
        byte_end,
        cut: span.cut,
        page_number: span.page_number,
        row_index: span.row_index,
        column_start: span.column_start,
        column_end: span.column_end,
        token_estimate: span.token_estimate,
        content_hash: chunk_content_hash(slice),
        survivor: None,
    })
}

fn push_kept(
    drafts: &mut Vec<Draft>,
    new_text: &str,
    chunk: &StoredChunk,
    cursor: usize,
) -> Result<usize, Error> {
    let length = chunk
        .byte_end
        .checked_sub(chunk.byte_start)
        .ok_or_else(violation)?;
    let byte_end = cursor.checked_add(length).ok_or_else(violation)?;
    let slice = new_text.get(cursor..byte_end).ok_or_else(violation)?;
    drafts.push(Draft {
        byte_start: cursor,
        byte_end,
        cut: chunk.cut,
        page_number: chunk.page_number,
        row_index: chunk.row_index,
        column_start: chunk.column_start,
        column_end: chunk.column_end,
        token_estimate: chunk.token_estimate,
        content_hash: chunk_content_hash(slice),
        survivor: Some(Survivor {
            chunk_id: chunk.chunk_id,
            occurrence: chunk.occurrence,
            ordinal: chunk.ordinal,
            recut: false,
        }),
    });
    Ok(byte_end)
}

fn claim_replacement(
    stored: &[StoredChunk],
    run: &Run,
    claimed: &mut [bool],
    content_hash: &[u8; 32],
) -> Option<Survivor> {
    for (offset, taken) in claimed.iter_mut().enumerate() {
        let candidate = &stored[run.first + offset];
        if !*taken && candidate.content_hash == *content_hash {
            *taken = true;
            return Some(Survivor {
                chunk_id: candidate.chunk_id,
                occurrence: candidate.occurrence,
                ordinal: candidate.ordinal,
                recut: true,
            });
        }
    }
    None
}

fn ensure_stored_tiling(stored: &[StoredChunk], old_text: &str) -> Result<(), Error> {
    let Some(first) = stored.first() else {
        return if old_text.is_empty() {
            Ok(())
        } else {
            Err(violation())
        };
    };
    if first.byte_start != 0 {
        return Err(violation());
    }
    for (position, chunk) in stored.iter().enumerate() {
        let ordinal = u32::try_from(position).map_err(|_| violation())?;
        if chunk.ordinal != ordinal
            || chunk.byte_end <= chunk.byte_start
            || chunk.byte_end > old_text.len()
            || !old_text.is_char_boundary(chunk.byte_start)
            || !old_text.is_char_boundary(chunk.byte_end)
        {
            return Err(violation());
        }
    }
    for pair in stored.windows(2) {
        if pair[1].byte_start != pair[0].byte_end {
            return Err(violation());
        }
    }
    if stored
        .last()
        .is_none_or(|chunk| chunk.byte_end != old_text.len())
    {
        return Err(violation());
    }
    Ok(())
}

fn merged_runs(regions: &[ChangedRegion], stored: &[StoredChunk], new_length: usize) -> Vec<Run> {
    let mut runs: Vec<Run> = Vec::new();
    for region in regions {
        let run = run_for(region, stored, new_length);
        match runs.last_mut() {
            Some(previous) if run.first <= previous.end => {
                previous.end = previous.end.max(run.end);
                previous.new_end = previous.new_end.max(run.new_end);
            }
            _ => runs.push(run),
        }
    }
    runs
}

fn run_for(region: &ChangedRegion, stored: &[StoredChunk], new_length: usize) -> Run {
    let overlaps = |chunk: &StoredChunk| {
        chunk.byte_start < region.old.end && chunk.byte_end > region.old.start
    };
    if let Some(first) = stored.iter().position(overlaps) {
        let last = stored.iter().rposition(overlaps).unwrap_or(first);
        return carried_run(
            region,
            first,
            last,
            &stored[first],
            &stored[last],
            new_length,
        );
    }
    let point = region.old.start;
    if let Some(inside) = stored
        .iter()
        .position(|chunk| chunk.byte_start < point && chunk.byte_end > point)
    {
        let chunk = &stored[inside];
        return carried_run(region, inside, inside, chunk, chunk, new_length);
    }
    let boundary = stored
        .iter()
        .position(|chunk| chunk.byte_start >= point)
        .unwrap_or(stored.len());
    Run {
        first: boundary,
        end: boundary,
        new_start: region.new.start,
        new_end: region.new.end,
    }
}

fn carried_run(
    region: &ChangedRegion,
    first: usize,
    last: usize,
    head: &StoredChunk,
    tail: &StoredChunk,
    new_length: usize,
) -> Run {
    let before = region.old.start.saturating_sub(head.byte_start);
    let after = tail.byte_end.saturating_sub(region.old.end);
    Run {
        first,
        end: last + 1,
        new_start: region.new.start.saturating_sub(before),
        new_end: region.new.end.saturating_add(after).min(new_length),
    }
}

fn ensure_lineage(chunk: &PlannedChunk, stored: &[StoredChunk]) -> Result<(), Error> {
    let known = match chunk.change {
        ChangeKind::Retained => stored
            .iter()
            .any(|kept| kept.chunk_id == chunk.chunk_id && kept.ordinal == chunk.ordinal),
        ChangeKind::Moved => stored
            .iter()
            .any(|kept| kept.chunk_id == chunk.chunk_id && kept.ordinal != chunk.ordinal),
        ChangeKind::Replaced => stored.iter().any(|kept| kept.chunk_id == chunk.chunk_id),
        ChangeKind::Added => true,
    };
    if known { Ok(()) } else { Err(violation()) }
}

fn violation() -> Error {
    Error::new(ErrorCode::InvariantViolation)
}
