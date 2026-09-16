#![allow(clippy::missing_errors_doc)]

use hm_core::{Error, ErrorCode};
use std::collections::BTreeMap;
use std::collections::btree_map::Entry;

const MAXIMUM_RELATION_SOURCES: usize = 4_096;
const MINIMUM_RELATION_TEXT_BYTES: usize = 16;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RelationSource {
    pub edge_id: Vec<u8>,
    pub edge_lsn: u64,
    pub source_id: Vec<u8>,
    pub target_id: Vec<u8>,
    pub relation: String,
    pub support_lsns: Vec<u64>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RelationRepresentation {
    pub relation_key: [u8; 32],
    pub edge_lsn: u64,
    pub text: String,
    pub support_lsns: Vec<u64>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RelationOptions {
    pub maximum_relations: usize,
    pub maximum_text_bytes: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RelationDrop {
    ZeroLsn,
    EmptyRelation,
    EmptyEndpoint,
    NoSupport,
    TextTooLong,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct RelationPlan {
    pub representations: Vec<RelationRepresentation>,
    pub dropped: Vec<(u64, RelationDrop)>,
}

pub fn represent(
    sources: &[RelationSource],
    options: RelationOptions,
) -> Result<RelationPlan, Error> {
    if options.maximum_relations == 0 || options.maximum_text_bytes < MINIMUM_RELATION_TEXT_BYTES {
        return Err(Error::new(ErrorCode::InvalidArgument));
    }
    if sources.len() > MAXIMUM_RELATION_SOURCES {
        return Err(Error::new(ErrorCode::CapacityExceeded));
    }
    let mut plan = RelationPlan::default();
    let mut accepted: BTreeMap<[u8; 32], RelationRepresentation> = BTreeMap::new();
    for source in sources {
        match representation(source, options) {
            Ok(representation) => collapse(&mut accepted, representation),
            Err(reason) => plan.dropped.push((source.edge_lsn, reason)),
        }
    }
    let mut representations: Vec<RelationRepresentation> = accepted.into_values().collect();
    representations.sort_by(|left, right| {
        left.edge_lsn
            .cmp(&right.edge_lsn)
            .then_with(|| left.relation_key.cmp(&right.relation_key))
    });
    representations.truncate(options.maximum_relations);
    plan.representations = representations;
    Ok(plan)
}

#[must_use]
pub fn relation_key(
    edge_id: &[u8],
    source_id: &[u8],
    relation: &str,
    target_id: &[u8],
) -> [u8; 32] {
    let mut hash = blake3::Hasher::new();
    hash.update(b"hypermind.relation-representation.v1\0");
    hash.update(edge_id);
    hash.update(&[0]);
    hash.update(source_id);
    hash.update(&[0]);
    hash.update(relation.as_bytes());
    hash.update(&[0]);
    hash.update(target_id);
    *hash.finalize().as_bytes()
}

fn representation(
    source: &RelationSource,
    options: RelationOptions,
) -> Result<RelationRepresentation, RelationDrop> {
    if source.edge_lsn == 0 {
        return Err(RelationDrop::ZeroLsn);
    }
    let relation = source.relation.trim();
    if relation.is_empty() {
        return Err(RelationDrop::EmptyRelation);
    }
    let head = label(&source.source_id);
    let tail = label(&source.target_id);
    if head.is_empty() || tail.is_empty() {
        return Err(RelationDrop::EmptyEndpoint);
    }
    let support = support(&source.support_lsns);
    if support.is_empty() {
        return Err(RelationDrop::NoSupport);
    }
    let text = format!("{head} {relation} {tail}");
    if text.len() > options.maximum_text_bytes {
        return Err(RelationDrop::TextTooLong);
    }
    Ok(RelationRepresentation {
        relation_key: relation_key(
            &source.edge_id,
            &source.source_id,
            &source.relation,
            &source.target_id,
        ),
        edge_lsn: source.edge_lsn,
        text,
        support_lsns: support,
    })
}

fn collapse(
    accepted: &mut BTreeMap<[u8; 32], RelationRepresentation>,
    incoming: RelationRepresentation,
) {
    match accepted.entry(incoming.relation_key) {
        Entry::Vacant(slot) => {
            slot.insert(incoming);
        }
        Entry::Occupied(mut slot) => {
            let existing = slot.get_mut();
            let mut merged = std::mem::take(&mut existing.support_lsns);
            merged.extend_from_slice(&incoming.support_lsns);
            merged.sort_unstable();
            merged.dedup();
            existing.support_lsns = merged;
            if incoming.edge_lsn >= existing.edge_lsn {
                existing.edge_lsn = incoming.edge_lsn;
                existing.text = incoming.text;
            }
        }
    }
}

fn label(id: &[u8]) -> String {
    String::from_utf8_lossy(id).trim().to_owned()
}

fn support(lsns: &[u64]) -> Vec<u64> {
    let mut support: Vec<u64> = lsns.iter().copied().filter(|lsn| *lsn != 0).collect();
    support.sort_unstable();
    support.dedup();
    support
}
