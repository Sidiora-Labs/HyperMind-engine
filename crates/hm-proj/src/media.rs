#![allow(clippy::missing_errors_doc)]

use crate::checkpoint::verify_frame;
use crate::generation::{decode, encode};
use crate::store::{Mutation, ProjectionId, ProjectionStore, ReadSnapshot};
use hm_core::{Error, ErrorCode};
use hm_ledger::frame::{EventKind, Frame};
use hm_schema::events::{Authority, EventPayload};
use serde::{Deserialize, Serialize};

const RECORD_PREFIX: u8 = b'M';
const PENDING_PREFIX: u8 = b'P';
const CALL_PREFIX: u8 = b'C';

const DERIVATION_DOMAIN: &[u8] = b"hypermind.media-derivation.v1\0";

pub const TRANSCRIPT_KIND_BYTE: u8 = 1;
pub const DESCRIPTION_KIND_BYTE: u8 = 2;
pub const DIGEST_BYTES: usize = 32;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct MediaRecord {
    pub digest: Vec<u8>,
    pub media_ref_lsn: u64,
    pub retained_lsn: u64,
    pub uri: String,
    pub media_type: String,
    pub derived_lsn: u64,
    pub derived_prompt_id: String,
}

pub struct MediaCatalogProjection;

impl MediaCatalogProjection {
    pub fn apply_event(store: &ProjectionStore, frame: &Frame) -> Result<(), Error> {
        match frame.header.kind {
            EventKind::MediaRef => apply_media_ref(store, frame),
            EventKind::ProviderFrame => apply_provider_frame(store, frame),
            EventKind::UserMsg => apply_derivation(store, frame),
            _ => store.apply(ProjectionId::MediaCatalog, frame.header.lsn, &[]),
        }
    }

    pub fn get(snapshot: &ReadSnapshot<'_>, digest: &[u8]) -> Result<Option<MediaRecord>, Error> {
        if digest.is_empty() {
            return Err(Error::new(ErrorCode::InvalidArgument));
        }
        snapshot
            .get(ProjectionId::MediaCatalog, &record_key(digest))?
            .map(|bytes| decode(&bytes))
            .transpose()
    }

    pub fn list(snapshot: &ReadSnapshot<'_>, limit: usize) -> Result<Vec<MediaRecord>, Error> {
        if limit == 0 {
            return Err(Error::new(ErrorCode::InvalidArgument));
        }
        snapshot
            .scan_prefix(ProjectionId::MediaCatalog, &[RECORD_PREFIX], limit)?
            .into_iter()
            .map(|entry| decode(&entry.value))
            .collect()
    }

    pub fn pending(snapshot: &ReadSnapshot<'_>, limit: usize) -> Result<Vec<MediaRecord>, Error> {
        if limit == 0 {
            return Err(Error::new(ErrorCode::InvalidArgument));
        }
        let markers = snapshot.scan_prefix(ProjectionId::MediaCatalog, &[PENDING_PREFIX], limit)?;
        let mut records = Vec::with_capacity(markers.len());
        for marker in markers {
            if let Some(record) = Self::get(snapshot, &marker.key[1..])? {
                records.push(record);
            }
        }
        Ok(records)
    }
}

#[must_use]
pub fn derivation_call_id(digest: &[u8; DIGEST_BYTES], kind_byte: u8) -> [u8; 32] {
    let mut hash = blake3::Hasher::new();
    hash.update(DERIVATION_DOMAIN);
    hash.update(digest);
    hash.update(&[kind_byte]);
    *hash.finalize().as_bytes()
}

fn apply_media_ref(store: &ProjectionStore, frame: &Frame) -> Result<(), Error> {
    let snapshot = store.begin_snapshot()?;
    let envelope = verify_frame(&snapshot, frame)?;
    let EventPayload::MediaRef(value) = envelope.payload else {
        return Err(Error::new(ErrorCode::InvalidKind).at_lsn(frame.header.lsn));
    };
    let media = *value;
    let digest = media.digest;
    let record = MediaRecord {
        digest: digest.clone(),
        media_ref_lsn: frame.header.lsn.get(),
        retained_lsn: 0,
        uri: media.uri,
        media_type: media.media_type,
        derived_lsn: 0,
        derived_prompt_id: String::new(),
    };
    let mut mutations = vec![
        Mutation::put(record_key(&digest), encode(&record)?),
        Mutation::put(pending_key(&digest), Vec::new()),
    ];
    if let Ok(fixed) = <[u8; DIGEST_BYTES]>::try_from(digest.as_slice()) {
        for kind_byte in [TRANSCRIPT_KIND_BYTE, DESCRIPTION_KIND_BYTE] {
            mutations.push(Mutation::put(
                call_key(&derivation_call_id(&fixed, kind_byte)),
                fixed.to_vec(),
            ));
        }
    }
    drop(snapshot);
    store.apply(ProjectionId::MediaCatalog, frame.header.lsn, &mutations)
}

fn apply_provider_frame(store: &ProjectionStore, frame: &Frame) -> Result<(), Error> {
    let snapshot = store.begin_snapshot()?;
    let envelope = verify_frame(&snapshot, frame)?;
    let EventPayload::ProviderFrame(value) = envelope.payload else {
        return Err(Error::new(ErrorCode::InvalidKind).at_lsn(frame.header.lsn));
    };
    let digest = blake3::hash(&value.api_content);
    let key = record_key(digest.as_bytes());
    let mut mutations = Vec::new();
    if let Some(bytes) = snapshot.get(ProjectionId::MediaCatalog, &key)? {
        let mut record = decode::<MediaRecord>(&bytes)?;
        record.retained_lsn = frame.header.lsn.get();
        mutations.push(Mutation::put(key, encode(&record)?));
    }
    drop(snapshot);
    store.apply(ProjectionId::MediaCatalog, frame.header.lsn, &mutations)
}

fn apply_derivation(store: &ProjectionStore, frame: &Frame) -> Result<(), Error> {
    let snapshot = store.begin_snapshot()?;
    let envelope = verify_frame(&snapshot, frame)?;
    let mut mutations = Vec::new();
    if envelope.authority == Authority::DerivedInference
        && let Some(model) = envelope.model_provenance.as_ref()
        && let Some(call_id) = model
            .call_id
            .as_deref()
            .filter(|call_id| call_id.len() == DIGEST_BYTES)
        && let Some(digest) = snapshot.get(ProjectionId::MediaCatalog, &call_key(call_id))?
    {
        let key = record_key(&digest);
        if let Some(bytes) = snapshot.get(ProjectionId::MediaCatalog, &key)? {
            let mut record = decode::<MediaRecord>(&bytes)?;
            record.derived_lsn = frame.header.lsn.get();
            record.derived_prompt_id.clone_from(&model.prompt_id);
            mutations.push(Mutation::put(key, encode(&record)?));
            mutations.push(Mutation::delete(pending_key(&digest)));
        }
    }
    drop(snapshot);
    store.apply(ProjectionId::MediaCatalog, frame.header.lsn, &mutations)
}

fn record_key(digest: &[u8]) -> Vec<u8> {
    prefixed_key(RECORD_PREFIX, digest)
}

fn pending_key(digest: &[u8]) -> Vec<u8> {
    prefixed_key(PENDING_PREFIX, digest)
}

fn call_key(call_id: &[u8]) -> Vec<u8> {
    prefixed_key(CALL_PREFIX, call_id)
}

fn prefixed_key(prefix: u8, tail: &[u8]) -> Vec<u8> {
    let mut key = Vec::with_capacity(tail.len() + 1);
    key.push(prefix);
    key.extend_from_slice(tail);
    key
}
