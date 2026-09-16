#![allow(clippy::missing_errors_doc)]

use crate::checkpoint::verify_frame;
use crate::generation::{decode, encode};
use crate::store::{Mutation, ProjectionId, ProjectionStore, ReadSnapshot};
use hm_core::{Error, ErrorCode};
use hm_index::vocabulary::{MAXIMUM_CANDIDATES, Q16_ONE, normalize_name, suggest_aliases};
use hm_ledger::frame::{EventKind, Frame};
use hm_schema::events::{EventPayload, VocabularyCategory};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

const HEAD_PREFIX: u8 = b'V';
const TERM_PREFIX: u8 = b'T';
const ALIAS_PREFIX: u8 = b'A';

pub const MAXIMUM_VOCABULARY_SCAN: usize = 4096;
pub const MAXIMUM_TERM_SCAN: usize = 65_536;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct VocabularyRecord {
    pub vocabulary_id: Vec<u8>,
    pub version: u16,
    pub source_uri: String,
    pub source_media_type: String,
    pub source_digest: Vec<u8>,
    pub term_count: u32,
    pub ignored_triples: u32,
    pub event_lsn: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TermRecord {
    pub vocabulary_id: Vec<u8>,
    pub version: u16,
    pub term_id: String,
    pub canonical_name: String,
    pub category: VocabularyCategory,
    pub parent_term_id: Option<String>,
    pub aliases: Vec<String>,
    pub event_lsn: u64,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct AliasEntry {
    pub vocabulary_id: Vec<u8>,
    pub version: u16,
    pub term_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AliasProposal {
    pub vocabulary_id: Vec<u8>,
    pub version: u16,
    pub term_id: String,
    pub canonical_name: String,
    pub category: VocabularyCategory,
    pub matched_candidate: String,
    pub similarity_q16: u32,
}

pub struct VocabularyProjection;

impl VocabularyProjection {
    pub fn apply_event(store: &ProjectionStore, frame: &Frame) -> Result<(), Error> {
        if frame.header.kind != EventKind::VocabularyImported {
            return store.apply(ProjectionId::Vocabulary, frame.header.lsn, &[]);
        }
        let snapshot = store.begin_snapshot()?;
        let envelope = verify_frame(&snapshot, frame)?;
        let EventPayload::VocabularyImported(value) = envelope.payload else {
            return Err(Error::new(ErrorCode::InvalidKind).at_lsn(frame.header.lsn));
        };
        let value = *value;
        let head = head_key(&value.vocabulary_id);
        let superseded = snapshot
            .get(ProjectionId::Vocabulary, &head)?
            .map(|bytes| decode::<VocabularyRecord>(&bytes))
            .transpose()?
            .is_some_and(|record| record.version >= value.version);
        if superseded {
            drop(snapshot);
            return store.apply(ProjectionId::Vocabulary, frame.header.lsn, &[]);
        }
        let record = VocabularyRecord {
            vocabulary_id: value.vocabulary_id.clone(),
            version: value.version,
            source_uri: value.source_uri,
            source_media_type: value.source_media_type,
            source_digest: value.source_digest,
            term_count: u32::try_from(value.terms.len())
                .map_err(|_| Error::new(ErrorCode::CapacityExceeded).at_lsn(frame.header.lsn))?,
            ignored_triples: value.ignored_triples,
            event_lsn: frame.header.lsn.get(),
        };
        let mut mutations = vec![Mutation::put(head, encode(&record)?)];
        let mut postings: BTreeMap<[u8; 33], Vec<AliasEntry>> = BTreeMap::new();
        for term in value.terms {
            let record = TermRecord {
                vocabulary_id: value.vocabulary_id.clone(),
                version: value.version,
                term_id: term.term_id,
                canonical_name: term.canonical_name,
                category: term.category,
                parent_term_id: term.parent_term_id,
                aliases: term.aliases.unwrap_or_default(),
                event_lsn: frame.header.lsn.get(),
            };
            let entry = AliasEntry {
                vocabulary_id: value.vocabulary_id.clone(),
                version: value.version,
                term_id: record.term_id.clone(),
            };
            for name in std::iter::once(&record.canonical_name).chain(record.aliases.iter()) {
                let normalized = normalize_name(name);
                if normalized.is_empty() {
                    continue;
                }
                postings
                    .entry(alias_key(&normalized))
                    .or_default()
                    .push(entry.clone());
            }
            mutations.push(Mutation::put(
                term_key(&value.vocabulary_id, value.version, &record.term_id),
                encode(&record)?,
            ));
        }
        for (key, mut entries) in postings {
            let mut posting = snapshot
                .get(ProjectionId::Vocabulary, &key)?
                .map(|bytes| decode::<Vec<AliasEntry>>(&bytes))
                .transpose()?
                .unwrap_or_default();
            posting.append(&mut entries);
            posting.sort();
            posting.dedup();
            mutations.push(Mutation::put(key, encode(&posting)?));
        }
        drop(snapshot);
        store.apply(ProjectionId::Vocabulary, frame.header.lsn, &mutations)
    }

    pub fn vocabulary(
        snapshot: &ReadSnapshot<'_>,
        vocabulary_id: &[u8],
    ) -> Result<Option<VocabularyRecord>, Error> {
        if vocabulary_id.is_empty() {
            return Err(Error::new(ErrorCode::InvalidArgument));
        }
        snapshot
            .get(ProjectionId::Vocabulary, &head_key(vocabulary_id))?
            .map(|bytes| decode(&bytes))
            .transpose()
    }

    pub fn list(snapshot: &ReadSnapshot<'_>, limit: usize) -> Result<Vec<VocabularyRecord>, Error> {
        if limit == 0 {
            return Err(Error::new(ErrorCode::InvalidArgument));
        }
        let mut records = heads(snapshot)?;
        records.sort_by(|left, right| left.vocabulary_id.cmp(&right.vocabulary_id));
        records.truncate(limit);
        Ok(records)
    }

    pub fn terms(snapshot: &ReadSnapshot<'_>, limit: usize) -> Result<Vec<TermRecord>, Error> {
        if limit == 0 {
            return Err(Error::new(ErrorCode::InvalidArgument));
        }
        let versions = head_versions(snapshot)?;
        let mut records: Vec<TermRecord> = snapshot
            .scan_prefix(ProjectionId::Vocabulary, &[TERM_PREFIX], MAXIMUM_TERM_SCAN)?
            .into_iter()
            .map(|entry| decode::<TermRecord>(&entry.value))
            .collect::<Result<Vec<_>, Error>>()?
            .into_iter()
            .filter(|record| versions.get(&record.vocabulary_id) == Some(&record.version))
            .collect();
        records.sort_by(|left, right| {
            left.vocabulary_id
                .cmp(&right.vocabulary_id)
                .then_with(|| left.term_id.cmp(&right.term_id))
        });
        records.truncate(limit);
        Ok(records)
    }

    pub fn resolve(
        snapshot: &ReadSnapshot<'_>,
        name: &str,
        limit: usize,
    ) -> Result<Vec<TermRecord>, Error> {
        if limit == 0 {
            return Err(Error::new(ErrorCode::InvalidArgument));
        }
        let normalized = normalize_name(name);
        if normalized.is_empty() {
            return Ok(Vec::new());
        }
        let Some(bytes) = snapshot.get(ProjectionId::Vocabulary, &alias_key(&normalized))? else {
            return Ok(Vec::new());
        };
        let posting = decode::<Vec<AliasEntry>>(&bytes)?;
        let mut versions: BTreeMap<Vec<u8>, u16> = BTreeMap::new();
        let mut records = Vec::new();
        for entry in posting {
            let version = if let Some(version) = versions.get(&entry.vocabulary_id) {
                *version
            } else {
                let version = Self::vocabulary(snapshot, &entry.vocabulary_id)?
                    .map_or(0, |record| record.version);
                versions.insert(entry.vocabulary_id.clone(), version);
                version
            };
            if version != entry.version {
                continue;
            }
            let key = term_key(&entry.vocabulary_id, entry.version, &entry.term_id);
            if let Some(bytes) = snapshot.get(ProjectionId::Vocabulary, &key)? {
                records.push(decode::<TermRecord>(&bytes)?);
            }
        }
        records.sort_by(|left, right| {
            left.vocabulary_id
                .cmp(&right.vocabulary_id)
                .then_with(|| left.term_id.cmp(&right.term_id))
        });
        records.truncate(limit);
        Ok(records)
    }

    pub fn suggest(
        snapshot: &ReadSnapshot<'_>,
        observed_name: &str,
        threshold_q16: u32,
        limit: usize,
    ) -> Result<Vec<AliasProposal>, Error> {
        if limit == 0 || threshold_q16 == 0 || threshold_q16 > Q16_ONE {
            return Err(Error::new(ErrorCode::InvalidArgument));
        }
        let terms = Self::terms(snapshot, MAXIMUM_TERM_SCAN)?;
        let mut candidates: Vec<Vec<String>> = Vec::with_capacity(terms.len());
        let mut total = 0;
        for term in &terms {
            let mut names: Vec<String> = std::iter::once(&term.canonical_name)
                .chain(term.aliases.iter())
                .map(String::as_str)
                .map(normalize_name)
                .filter(|name| !name.is_empty())
                .collect();
            names.sort();
            names.dedup();
            total += names.len();
            candidates.push(names);
        }
        if total > MAXIMUM_CANDIDATES {
            return Err(Error::new(ErrorCode::InvalidArgument));
        }
        let mut proposals = Vec::new();
        for (term, names) in terms.iter().zip(candidates.iter()) {
            let names: Vec<&str> = names.iter().map(String::as_str).collect();
            if names.is_empty() {
                continue;
            }
            let Some(best) = suggest_aliases(observed_name, &names, threshold_q16, 1)?
                .into_iter()
                .next()
            else {
                continue;
            };
            proposals.push(AliasProposal {
                vocabulary_id: term.vocabulary_id.clone(),
                version: term.version,
                term_id: term.term_id.clone(),
                canonical_name: term.canonical_name.clone(),
                category: term.category,
                matched_candidate: best.candidate,
                similarity_q16: best.similarity_q16,
            });
        }
        proposals.sort_by(|left, right| {
            right
                .similarity_q16
                .cmp(&left.similarity_q16)
                .then_with(|| left.vocabulary_id.cmp(&right.vocabulary_id))
                .then_with(|| left.term_id.cmp(&right.term_id))
        });
        proposals.truncate(limit);
        Ok(proposals)
    }
}

fn heads(snapshot: &ReadSnapshot<'_>) -> Result<Vec<VocabularyRecord>, Error> {
    snapshot
        .scan_prefix(
            ProjectionId::Vocabulary,
            &[HEAD_PREFIX],
            MAXIMUM_VOCABULARY_SCAN,
        )?
        .into_iter()
        .map(|entry| decode(&entry.value))
        .collect()
}

fn head_versions(snapshot: &ReadSnapshot<'_>) -> Result<BTreeMap<Vec<u8>, u16>, Error> {
    Ok(heads(snapshot)?
        .into_iter()
        .map(|record| (record.vocabulary_id, record.version))
        .collect())
}

fn head_key(vocabulary_id: &[u8]) -> [u8; 33] {
    digest_key(HEAD_PREFIX, blake3::hash(vocabulary_id).as_bytes())
}

fn term_key(vocabulary_id: &[u8], version: u16, term_id: &str) -> [u8; 33] {
    let mut hasher = blake3::Hasher::new();
    hasher.update(vocabulary_id);
    hasher.update(&[0]);
    hasher.update(&version.to_be_bytes());
    hasher.update(term_id.as_bytes());
    digest_key(TERM_PREFIX, hasher.finalize().as_bytes())
}

fn alias_key(normalized_name: &str) -> [u8; 33] {
    digest_key(
        ALIAS_PREFIX,
        blake3::hash(normalized_name.as_bytes()).as_bytes(),
    )
}

fn digest_key(prefix: u8, digest: &[u8; 32]) -> [u8; 33] {
    let mut key = [0; 33];
    key[0] = prefix;
    key[1..].copy_from_slice(digest);
    key
}
