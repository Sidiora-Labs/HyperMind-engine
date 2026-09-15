#![allow(clippy::missing_errors_doc)]

use crate::store::{Mutation, ProjectionId, ProjectionStore, ReadSnapshot};
use hm_core::{Error, ErrorCode, LSN};
use hm_index::entity_rules::{extract_entities, extract_query_entities};
use hm_ledger::frame::{EventKind, Frame};
use hm_schema::event::{self, Boundary};
use hm_schema::events::EventPayload;
use roaring::RoaringTreemap;
use std::collections::{BTreeMap, BTreeSet};
use std::io::Cursor;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EntityHit {
    pub lsn: LSN,
    pub matched_aliases: Vec<String>,
}

impl EntityHit {
    #[must_use]
    pub const fn score(&self) -> usize {
        self.matched_aliases.len()
    }
}

pub struct EntityProjection;

impl EntityProjection {
    pub fn apply_event(store: &ProjectionStore, frame: &Frame) -> Result<(), Error> {
        let Some(text) = document_text(frame)? else {
            return store.apply(ProjectionId::EntityIndex, frame.header.lsn, &[]);
        };
        let aliases: BTreeSet<_> = extract_entities(&text)
            .into_iter()
            .flat_map(|entity| entity.aliases)
            .collect();
        let snapshot = store.begin_snapshot()?;
        let mut mutations = Vec::with_capacity(aliases.len());
        for alias in aliases {
            let key = posting_key(&alias);
            let mut postings = decode_bitmap(snapshot.get(ProjectionId::EntityIndex, &key)?)?;
            postings.insert(frame.header.lsn.get());
            mutations.push(Mutation::put(key, encode_bitmap(&postings)?));
        }
        drop(snapshot);
        store.apply(ProjectionId::EntityIndex, frame.header.lsn, &mutations)
    }

    pub fn query(
        snapshot: &ReadSnapshot<'_>,
        query: &str,
        turn_text: &str,
        limit: usize,
    ) -> Result<Vec<EntityHit>, Error> {
        if limit == 0 {
            return Err(Error::new(ErrorCode::InvalidArgument));
        }
        let aliases: BTreeSet<_> = extract_query_entities(query, turn_text)
            .into_iter()
            .flat_map(|entity| entity.aliases)
            .collect();
        let mut matches = BTreeMap::<u64, Vec<String>>::new();
        for alias in aliases {
            let postings =
                decode_bitmap(snapshot.get(ProjectionId::EntityIndex, &posting_key(&alias))?)?;
            for lsn in postings {
                matches.entry(lsn).or_default().push(alias.clone());
            }
        }
        let mut hits: Vec<_> = matches
            .into_iter()
            .map(|(lsn, mut matched_aliases)| {
                matched_aliases.sort();
                matched_aliases.dedup();
                EntityHit {
                    lsn: LSN::new(lsn),
                    matched_aliases,
                }
            })
            .collect();
        hits.sort_by(|left, right| {
            right
                .score()
                .cmp(&left.score())
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

fn posting_key(alias: &str) -> Vec<u8> {
    let mut key = Vec::with_capacity(alias.len() + 1);
    key.push(b'P');
    key.extend_from_slice(alias.as_bytes());
    key
}

fn decode_bitmap(value: Option<Vec<u8>>) -> Result<RoaringTreemap, Error> {
    match value {
        None => Ok(RoaringTreemap::new()),
        Some(bytes) => RoaringTreemap::deserialize_from(Cursor::new(bytes))
            .map_err(|_| Error::new(ErrorCode::EntityIndexCorrupt)),
    }
}

fn encode_bitmap(bitmap: &RoaringTreemap) -> Result<Vec<u8>, Error> {
    let mut bytes = Vec::new();
    bitmap
        .serialize_into(&mut bytes)
        .map_err(|_| Error::new(ErrorCode::BackendUnavailable))?;
    Ok(bytes)
}
