#![allow(clippy::missing_errors_doc)]

use core::ops::Range;

use hm_core::{Error, ErrorCode};

use crate::loader::{PageSpan, RowSpan, TableLayout};

pub const MAXIMUM_TOKEN_BUDGET: u32 = 65_536;

const TABLE_PAGE_NUMBER: u32 = 1;

pub trait TokenEstimator {
    fn estimate(&self, text: &str) -> u32;
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct WordEstimator;

impl TokenEstimator for WordEstimator {
    fn estimate(&self, text: &str) -> u32 {
        u32::try_from(text.split_whitespace().count()).unwrap_or(u32::MAX)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChunkCut {
    ParagraphEnd,
    ParagraphCut,
    RowEnd,
    RowCut,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ChunkSpan {
    pub byte_start: usize,
    pub byte_end: usize,
    pub cut: ChunkCut,
    pub token_estimate: u32,
    pub page_number: u32,
    pub row_index: u32,
    pub column_start: u32,
    pub column_end: u32,
}

#[must_use]
pub fn tiles(spans: &[ChunkSpan], length: usize) -> bool {
    let Some(first) = spans.first() else {
        return length == 0;
    };
    if first.byte_start != 0 {
        return false;
    }
    for span in spans {
        if span.byte_end <= span.byte_start {
            return false;
        }
    }
    for pair in spans.windows(2) {
        if pair[1].byte_start != pair[0].byte_end {
            return false;
        }
    }
    spans.last().is_some_and(|span| span.byte_end == length)
}

#[must_use]
pub fn paragraph_units(text: &str) -> Vec<Range<usize>> {
    let bytes = text.as_bytes();
    let mut units = Vec::new();
    let mut start = 0;
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] != b'\n' {
            index += 1;
            continue;
        }
        let mut end = index + 1;
        let mut terminators = 1;
        loop {
            if bytes.get(end) == Some(&b'\n') {
                end += 1;
            } else if bytes.get(end) == Some(&b'\r') && bytes.get(end + 1) == Some(&b'\n') {
                end += 2;
            } else {
                break;
            }
            terminators += 1;
        }
        if terminators >= 2 {
            units.push(start..end);
            start = end;
        }
        index = end;
    }
    if start < bytes.len() {
        units.push(start..bytes.len());
    }
    units
}

pub fn chunk_paragraphs(
    text: &str,
    budget: u32,
    estimator: &dyn TokenEstimator,
    pages: &[PageSpan],
) -> Result<Vec<ChunkSpan>, Error> {
    guard_budget(budget)?;
    if text.is_empty() {
        return Ok(Vec::new());
    }
    let mut cuts: Vec<(Range<usize>, ChunkCut)> = Vec::new();
    let mut batch: Option<(usize, usize, u32)> = None;
    for unit in paragraph_units(text) {
        let estimate = estimator.estimate(&text[unit.clone()]);
        if estimate > budget {
            if let Some((start, end, _)) = batch.take() {
                cuts.push((start..end, ChunkCut::ParagraphEnd));
            }
            let pieces = split_unit(text, &unit, budget, estimator);
            let last = pieces.len().saturating_sub(1);
            for (index, piece) in pieces.into_iter().enumerate() {
                let cut = if index == last {
                    ChunkCut::ParagraphEnd
                } else {
                    ChunkCut::ParagraphCut
                };
                cuts.push((piece, cut));
            }
            continue;
        }
        batch = match batch {
            None => Some((unit.start, unit.end, estimate)),
            Some((start, _, carried)) if carried.saturating_add(estimate) <= budget => {
                Some((start, unit.end, carried.saturating_add(estimate)))
            }
            Some((start, end, _)) => {
                cuts.push((start..end, ChunkCut::ParagraphEnd));
                Some((unit.start, unit.end, estimate))
            }
        };
    }
    if let Some((start, end, _)) = batch {
        cuts.push((start..end, ChunkCut::ParagraphEnd));
    }
    let mut spans = Vec::with_capacity(cuts.len());
    for (range, cut) in cuts {
        spans.push(ChunkSpan {
            byte_start: range.start,
            byte_end: range.end,
            cut,
            token_estimate: estimator.estimate(&text[range.clone()]),
            page_number: page_number_at(pages, range.start),
            row_index: 0,
            column_start: 0,
            column_end: 0,
        });
    }
    Ok(spans)
}

pub fn chunk_table(
    text: &str,
    layout: &TableLayout,
    budget: u32,
    estimator: &dyn TokenEstimator,
) -> Result<Vec<ChunkSpan>, Error> {
    guard_budget(budget)?;
    if text.is_empty() {
        return Ok(Vec::new());
    }
    let header_end = layout.rows.first().map_or(text.len(), |row| row.byte_start);
    if header_end > text.len() || !text.is_char_boundary(header_end) {
        return Err(Error::new(ErrorCode::InvalidArgument));
    }
    let mut spans = Vec::new();
    let mut cursor = 0;
    if header_end > 0 {
        spans.push(ChunkSpan {
            byte_start: 0,
            byte_end: header_end,
            cut: ChunkCut::RowEnd,
            token_estimate: estimator.estimate(&text[..header_end]),
            page_number: TABLE_PAGE_NUMBER,
            row_index: 0,
            column_start: 0,
            column_end: column_count(layout.header.len())?,
        });
        cursor = header_end;
    }
    for (index, row) in layout.rows.iter().enumerate() {
        let record_end = layout
            .rows
            .get(index + 1)
            .map_or(text.len(), |next| next.byte_start);
        if row.byte_start != cursor
            || record_end <= cursor
            || record_end > text.len()
            || !text.is_char_boundary(record_end)
        {
            return Err(Error::new(ErrorCode::InvalidArgument));
        }
        let pieces = record_pieces(text, row, cursor, record_end, budget, estimator)?;
        let last = pieces.len().saturating_sub(1);
        for (position, piece) in pieces.into_iter().enumerate() {
            let cut = if position == last {
                ChunkCut::RowEnd
            } else {
                ChunkCut::RowCut
            };
            spans.push(ChunkSpan {
                byte_start: piece.range.start,
                byte_end: piece.range.end,
                cut,
                token_estimate: estimator.estimate(&text[piece.range.clone()]),
                page_number: TABLE_PAGE_NUMBER,
                row_index: row.row_index,
                column_start: piece.column_start,
                column_end: piece.column_end,
            });
        }
        cursor = record_end;
    }
    Ok(spans)
}

struct RecordPiece {
    range: Range<usize>,
    column_start: u32,
    column_end: u32,
    estimate: u32,
}

fn record_pieces(
    text: &str,
    row: &RowSpan,
    record_start: usize,
    record_end: usize,
    budget: u32,
    estimator: &dyn TokenEstimator,
) -> Result<Vec<RecordPiece>, Error> {
    if row.fields.is_empty() {
        return Ok(vec![RecordPiece {
            range: record_start..record_end,
            column_start: 0,
            column_end: 0,
            estimate: estimator.estimate(&text[record_start..record_end]),
        }]);
    }
    let mut pieces: Vec<RecordPiece> = Vec::with_capacity(row.fields.len());
    for (index, field) in row.fields.iter().enumerate() {
        let start = if index == 0 {
            record_start
        } else {
            field.byte_start
        };
        let end = row
            .fields
            .get(index + 1)
            .map_or(record_end, |next| next.byte_start);
        if start >= end
            || end > record_end
            || !text.is_char_boundary(start)
            || !text.is_char_boundary(end)
        {
            return Err(Error::new(ErrorCode::InvalidArgument));
        }
        let column_end = field
            .column_index
            .checked_add(1)
            .ok_or_else(|| Error::new(ErrorCode::CapacityExceeded))?;
        let estimate = estimator.estimate(&text[start..end]);
        match pieces.last_mut() {
            Some(previous)
                if previous.range.end == start
                    && previous.estimate.saturating_add(estimate) <= budget =>
            {
                previous.range.end = end;
                previous.column_end = column_end;
                previous.estimate = previous.estimate.saturating_add(estimate);
            }
            _ => pieces.push(RecordPiece {
                range: start..end,
                column_start: field.column_index,
                column_end,
                estimate,
            }),
        }
    }
    Ok(pieces)
}

fn split_unit(
    text: &str,
    unit: &Range<usize>,
    budget: u32,
    estimator: &dyn TokenEstimator,
) -> Vec<Range<usize>> {
    let mut pieces: Vec<Range<usize>> = Vec::new();
    let mut batch: Option<(usize, usize, u32)> = None;
    for sentence in sentence_units(text, unit) {
        let estimate = estimator.estimate(&text[sentence.clone()]);
        if estimate > budget {
            if let Some((start, end, _)) = batch.take() {
                pieces.push(start..end);
            }
            let mut start = sentence.start;
            while start < sentence.end {
                let end = hard_cut(text, start, sentence.end, budget, estimator);
                pieces.push(start..end);
                start = end;
            }
            continue;
        }
        batch = match batch {
            None => Some((sentence.start, sentence.end, estimate)),
            Some((start, _, carried)) if carried.saturating_add(estimate) <= budget => {
                Some((start, sentence.end, carried.saturating_add(estimate)))
            }
            Some((start, end, _)) => {
                pieces.push(start..end);
                Some((sentence.start, sentence.end, estimate))
            }
        };
    }
    if let Some((start, end, _)) = batch {
        pieces.push(start..end);
    }
    pieces
}

fn sentence_units(text: &str, unit: &Range<usize>) -> Vec<Range<usize>> {
    let bytes = text.as_bytes();
    let mut units = Vec::new();
    let mut start = unit.start;
    let mut index = unit.start;
    while index < unit.end {
        let byte = bytes[index];
        index += 1;
        if !matches!(byte, b'.' | b'!' | b'?') {
            continue;
        }
        while index < unit.end && matches!(bytes[index], b'.' | b'!' | b'?' | b'"' | b'\'' | b')') {
            index += 1;
        }
        let mut end = index;
        while end < unit.end && bytes[end].is_ascii_whitespace() {
            end += 1;
        }
        if end > index || end == unit.end {
            units.push(start..end);
            start = end;
            index = end;
        }
    }
    if start < unit.end {
        units.push(start..unit.end);
    }
    units
}

fn hard_cut(
    text: &str,
    start: usize,
    limit: usize,
    budget: u32,
    estimator: &dyn TokenEstimator,
) -> usize {
    if estimator.estimate(&text[start..limit]) <= budget {
        return limit;
    }
    let mut low = start;
    let mut high = limit;
    while low + 1 < high {
        let midpoint = low + (high - low) / 2;
        let Some(boundary) = interior_boundary(text, low, high, midpoint) else {
            break;
        };
        if estimator.estimate(&text[start..boundary]) <= budget {
            low = boundary;
        } else {
            high = boundary;
        }
    }
    if low > start {
        low
    } else {
        next_boundary(text, start, limit)
    }
}

fn interior_boundary(text: &str, low: usize, high: usize, midpoint: usize) -> Option<usize> {
    let mut below = midpoint;
    while below > low && !text.is_char_boundary(below) {
        below -= 1;
    }
    if below > low {
        return Some(below);
    }
    let mut above = midpoint + 1;
    while above < high && !text.is_char_boundary(above) {
        above += 1;
    }
    if above < high { Some(above) } else { None }
}

fn next_boundary(text: &str, start: usize, limit: usize) -> usize {
    let mut end = start + 1;
    while end < limit && !text.is_char_boundary(end) {
        end += 1;
    }
    end.min(limit)
}

fn page_number_at(pages: &[PageSpan], offset: usize) -> u32 {
    pages
        .iter()
        .find(|page| offset >= page.byte_start && offset < page.byte_end)
        .map_or(0, |page| page.page_number)
}

fn column_count(columns: usize) -> Result<u32, Error> {
    u32::try_from(columns).map_err(|_| Error::new(ErrorCode::CapacityExceeded))
}

fn guard_budget(budget: u32) -> Result<(), Error> {
    if budget == 0 || budget > MAXIMUM_TOKEN_BUDGET {
        return Err(Error::new(ErrorCode::InvalidArgument));
    }
    Ok(())
}
