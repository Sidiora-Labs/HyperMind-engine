#![allow(clippy::missing_errors_doc)]

use crate::store::{Mutation, ProjectionId, ProjectionStore, ReadSnapshot};
use hm_core::{Error, ErrorCode, LSN};
use hm_index::{bm25, tokenize};
use hm_ledger::frame::{EventKind, Frame};
use hm_schema::event::{self, Boundary};
use hm_schema::events::EventPayload;
use roaring::RoaringTreemap;
use std::collections::{BTreeMap, BTreeSet};
use std::io::Cursor;

const STATS_KEY: [u8; 1] = [b'S'];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LexicalHit {
    pub lsn: LSN,
    pub score_q32: u64,
}

pub struct LexicalProjection;

impl LexicalProjection {
    pub fn apply_event(store: &ProjectionStore, frame: &Frame) -> Result<(), Error> {
        let Some(text) = document_text(frame)? else {
            return store.apply(ProjectionId::Bm25, frame.header.lsn, &[]);
        };
        let terms = tokenize::tokenize(&text);
        if terms.is_empty() {
            return store.apply(ProjectionId::Bm25, frame.header.lsn, &[]);
        }
        let snapshot = store.begin_snapshot()?;
        let (document_count, total_terms) = read_stats(&snapshot)?;
        let mut frequencies = BTreeMap::<String, u32>::new();
        for term in &terms {
            let frequency = frequencies.entry(term.clone()).or_default();
            *frequency = frequency
                .checked_add(1)
                .ok_or_else(|| Error::new(ErrorCode::CapacityExceeded))?;
        }
        let mut mutations = Vec::with_capacity(frequencies.len() * 2 + 2);
        for (term, frequency) in frequencies {
            let posting_key = posting_key(&term);
            let mut postings = decode_bitmap(snapshot.get(ProjectionId::Bm25, &posting_key)?)?;
            postings.insert(frame.header.lsn.get());
            mutations.push(Mutation::put(posting_key, encode_bitmap(&postings)?));
            mutations.push(Mutation::put(
                frequency_key(&term, frame.header.lsn),
                frequency.to_le_bytes(),
            ));
        }
        let term_count =
            u32::try_from(terms.len()).map_err(|_| Error::new(ErrorCode::CapacityExceeded))?;
        mutations.push(Mutation::put(
            document_key(frame.header.lsn),
            term_count.to_le_bytes(),
        ));
        let mut stats = Vec::with_capacity(16);
        stats.extend_from_slice(
            &document_count
                .checked_add(1)
                .ok_or_else(|| Error::new(ErrorCode::CapacityExceeded))?
                .to_le_bytes(),
        );
        stats.extend_from_slice(
            &total_terms
                .checked_add(terms.len() as u64)
                .ok_or_else(|| Error::new(ErrorCode::CapacityExceeded))?
                .to_le_bytes(),
        );
        mutations.push(Mutation::put(STATS_KEY, stats));
        drop(snapshot);
        store.apply(ProjectionId::Bm25, frame.header.lsn, &mutations)
    }

    pub fn query(
        snapshot: &ReadSnapshot<'_>,
        query: &str,
        limit: usize,
    ) -> Result<Vec<LexicalHit>, Error> {
        let terms: BTreeSet<String> = tokenize::tokenize(query).into_iter().collect();
        if terms.is_empty() || limit == 0 {
            return Err(Error::new(ErrorCode::InvalidArgument));
        }
        let (document_count, total_terms) = read_stats(snapshot)?;
        if document_count == 0 {
            return Ok(Vec::new());
        }
        let mut scores = BTreeMap::<u64, u64>::new();
        for term in terms {
            let postings = decode_bitmap(snapshot.get(ProjectionId::Bm25, &posting_key(&term))?)?;
            let document_frequency = postings.len();
            if document_frequency == 0 {
                continue;
            }
            for lsn in postings {
                let term_frequency = read_u32(
                    snapshot
                        .get(ProjectionId::Bm25, &frequency_key(&term, LSN::new(lsn)))?
                        .as_deref(),
                )?;
                let document_length = read_u32(
                    snapshot
                        .get(ProjectionId::Bm25, &document_key(LSN::new(lsn)))?
                        .as_deref(),
                )?;
                let score = bm25::score_q32(
                    document_count,
                    document_frequency,
                    total_terms,
                    document_length,
                    term_frequency,
                )?;
                let accumulated = scores.entry(lsn).or_default();
                *accumulated = accumulated
                    .checked_add(score)
                    .ok_or_else(|| Error::new(ErrorCode::CapacityExceeded))?;
            }
        }
        let mut hits: Vec<LexicalHit> = scores
            .into_iter()
            .map(|(lsn, score_q32)| LexicalHit {
                lsn: LSN::new(lsn),
                score_q32,
            })
            .collect();
        hits.sort_unstable_by(|left, right| {
            right
                .score_q32
                .cmp(&left.score_q32)
                .then_with(|| left.lsn.cmp(&right.lsn))
        });
        hits.truncate(limit);
        Ok(hits)
    }
}

fn document_text(frame: &Frame) -> Result<Option<String>, Error> {
    let schema_kind = match frame.header.kind {
        EventKind::UserMsg => event::EventKind::UserMsg,
        EventKind::DeliveredMsg => event::EventKind::DeliveredMsg,
        _ => return Ok(None),
    };
    let verified = event::verify_event(&frame.sealed_payload, schema_kind, Boundary::Disk)
        .map_err(|error| error.at_lsn(frame.header.lsn))?;
    let bytes = match verified.envelope.payload {
        EventPayload::UserMsg(message) => message.content,
        EventPayload::DeliveredMsg(message) => message.content,
        _ => return Err(Error::new(ErrorCode::InvariantViolation).at_lsn(frame.header.lsn)),
    };
    String::from_utf8(bytes)
        .map(Some)
        .map_err(|_| Error::new(ErrorCode::SchemaInvalid).at_lsn(frame.header.lsn))
}

fn posting_key(term: &str) -> Vec<u8> {
    let mut key = Vec::with_capacity(term.len() + 1);
    key.push(b'P');
    key.extend_from_slice(term.as_bytes());
    key
}

fn frequency_key(term: &str, lsn: LSN) -> Vec<u8> {
    let mut key = posting_key(term);
    key[0] = b'F';
    key.push(0);
    key.extend_from_slice(&lsn.get().to_be_bytes());
    key
}

fn document_key(lsn: LSN) -> [u8; 9] {
    let mut key = [0_u8; 9];
    key[0] = b'D';
    key[1..].copy_from_slice(&lsn.get().to_be_bytes());
    key
}

fn read_stats(snapshot: &ReadSnapshot<'_>) -> Result<(u64, u64), Error> {
    let Some(bytes) = snapshot.get(ProjectionId::Bm25, &STATS_KEY)? else {
        return Ok((0, 0));
    };
    if bytes.len() != 16 {
        return Err(Error::new(ErrorCode::LexicalIndexCorrupt));
    }
    Ok((
        u64::from_le_bytes(copy_array(&bytes, 0)?),
        u64::from_le_bytes(copy_array(&bytes, 8)?),
    ))
}

fn read_u32(value: Option<&[u8]>) -> Result<u32, Error> {
    let bytes = value.ok_or_else(|| Error::new(ErrorCode::LexicalIndexCorrupt))?;
    if bytes.len() != 4 {
        return Err(Error::new(ErrorCode::LexicalIndexCorrupt));
    }
    Ok(u32::from_le_bytes(copy_array(bytes, 0)?))
}

fn decode_bitmap(value: Option<Vec<u8>>) -> Result<RoaringTreemap, Error> {
    match value {
        None => Ok(RoaringTreemap::new()),
        Some(bytes) => RoaringTreemap::deserialize_from(Cursor::new(bytes))
            .map_err(|_| Error::new(ErrorCode::LexicalIndexCorrupt)),
    }
}

fn encode_bitmap(bitmap: &RoaringTreemap) -> Result<Vec<u8>, Error> {
    let mut bytes = Vec::new();
    bitmap
        .serialize_into(&mut bytes)
        .map_err(|_| Error::new(ErrorCode::BackendUnavailable))?;
    Ok(bytes)
}

fn copy_array<const N: usize>(bytes: &[u8], offset: usize) -> Result<[u8; N], Error> {
    bytes
        .get(offset..offset + N)
        .ok_or_else(|| Error::new(ErrorCode::LexicalIndexCorrupt))?
        .try_into()
        .map_err(|_| Error::new(ErrorCode::LexicalIndexCorrupt))
}
