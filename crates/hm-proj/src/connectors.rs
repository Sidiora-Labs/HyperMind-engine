#![allow(clippy::missing_errors_doc)]

use crate::checkpoint::verify_frame;
use crate::generation::{decode, encode};
use crate::store::{Mutation, ProjectionId, ProjectionStore, ReadSnapshot};
use hm_core::{Error, ErrorCode};
use hm_ledger::frame::{EventKind, Frame};
use hm_schema::events::{
    EventPayload, SourceConnectorBound, SourceDeliveryAccepted, SourceDeliverySettled,
    SourceDeliveryState, SourceRevisionObserved,
};
use serde::{Deserialize, Serialize};

const CONNECTOR_PREFIX: u8 = b'C';
const DELIVERY_PREFIX: u8 = b'D';
const REVISION_PREFIX: u8 = b'R';
const NONCE_PREFIX: u8 = b'N';

pub const MAXIMUM_REGISTRY_SCAN: usize = 256;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ConnectorRecord {
    pub connector_id: Vec<u8>,
    pub provider: String,
    pub external_account: String,
    pub credential_version: u32,
    pub signature_scheme: u8,
    pub scopes: Vec<String>,
    pub state: u8,
    pub bound_lsn: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DeliveryRecord {
    pub connector_id: Vec<u8>,
    pub delivery_id: Vec<u8>,
    pub accepted_lsn: u64,
    pub body_digest: Vec<u8>,
    pub body_bytes: u64,
    pub event_name: String,
    pub attempt: u32,
    pub state: u8,
    pub next_attempt_at_ns: i64,
    pub detail: String,
    pub settled_lsn: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RevisionRecord {
    pub connector_id: Vec<u8>,
    pub source_id: String,
    pub revision: Vec<u8>,
    pub content_digest: Vec<u8>,
    pub observed_at_ns: i64,
    pub lsn: u64,
}

pub struct ConnectorRegistryProjection;

impl ConnectorRegistryProjection {
    pub fn apply_event(store: &ProjectionStore, frame: &Frame) -> Result<(), Error> {
        if !matches!(
            frame.header.kind,
            EventKind::SourceConnectorBound
                | EventKind::SourceDeliveryAccepted
                | EventKind::SourceDeliverySettled
                | EventKind::SourceRevisionObserved
        ) {
            return store.apply(ProjectionId::SourceConnectors, frame.header.lsn, &[]);
        }
        let snapshot = store.begin_snapshot()?;
        let envelope = verify_frame(&snapshot, frame)?;
        let lsn = frame.header.lsn.get();
        let mutations = match &envelope.payload {
            EventPayload::SourceConnectorBound(value) => bound_mutations(value, lsn)?,
            EventPayload::SourceDeliveryAccepted(value) => accepted_mutations(value, lsn)?,
            EventPayload::SourceDeliverySettled(value) => settled_mutations(&snapshot, value, lsn)?,
            EventPayload::SourceRevisionObserved(value) => revision_mutations(value, lsn)?,
            _ => return Err(Error::new(ErrorCode::InvalidKind).at_lsn(frame.header.lsn)),
        };
        drop(snapshot);
        store.apply(ProjectionId::SourceConnectors, frame.header.lsn, &mutations)
    }

    pub fn connector(
        snapshot: &ReadSnapshot<'_>,
        connector_id: &[u8],
    ) -> Result<Option<ConnectorRecord>, Error> {
        if connector_id.is_empty() {
            return Err(Error::new(ErrorCode::InvalidArgument));
        }
        snapshot
            .get(
                ProjectionId::SourceConnectors,
                &scoped_prefix(CONNECTOR_PREFIX, connector_id),
            )?
            .map(|bytes| decode(&bytes))
            .transpose()
    }

    pub fn delivery(
        snapshot: &ReadSnapshot<'_>,
        connector_id: &[u8],
        delivery_id: &[u8],
    ) -> Result<Option<DeliveryRecord>, Error> {
        if connector_id.is_empty() || delivery_id.is_empty() {
            return Err(Error::new(ErrorCode::InvalidArgument));
        }
        snapshot
            .get(
                ProjectionId::SourceConnectors,
                &delivery_key(connector_id, delivery_id),
            )?
            .map(|bytes| decode(&bytes))
            .transpose()
    }

    pub fn revision(
        snapshot: &ReadSnapshot<'_>,
        connector_id: &[u8],
        source_id: &str,
    ) -> Result<Option<RevisionRecord>, Error> {
        if connector_id.is_empty() || source_id.is_empty() {
            return Err(Error::new(ErrorCode::InvalidArgument));
        }
        snapshot
            .get(
                ProjectionId::SourceConnectors,
                &revision_key(connector_id, source_id),
            )?
            .map(|bytes| decode(&bytes))
            .transpose()
    }

    pub fn consent_redeemed(snapshot: &ReadSnapshot<'_>, nonce: &[u8]) -> Result<bool, Error> {
        if nonce.is_empty() {
            return Err(Error::new(ErrorCode::InvalidArgument));
        }
        Ok(snapshot
            .get(
                ProjectionId::SourceConnectors,
                &scoped_prefix(NONCE_PREFIX, nonce),
            )?
            .is_some())
    }

    pub fn list_connectors(
        snapshot: &ReadSnapshot<'_>,
        limit: usize,
    ) -> Result<Vec<ConnectorRecord>, Error> {
        let limit = bounded_limit(limit)?;
        snapshot
            .scan_prefix(ProjectionId::SourceConnectors, &[CONNECTOR_PREFIX], limit)?
            .into_iter()
            .map(|entry| decode(&entry.value))
            .collect()
    }

    pub fn recent_deliveries(
        snapshot: &ReadSnapshot<'_>,
        connector_id: &[u8],
        limit: usize,
    ) -> Result<Vec<DeliveryRecord>, Error> {
        let limit = bounded_limit(limit)?;
        if connector_id.is_empty() {
            return Err(Error::new(ErrorCode::InvalidArgument));
        }
        let mut records: Vec<DeliveryRecord> = snapshot
            .scan_prefix(
                ProjectionId::SourceConnectors,
                &scoped_prefix(DELIVERY_PREFIX, connector_id),
                MAXIMUM_REGISTRY_SCAN,
            )?
            .into_iter()
            .map(|entry| decode(&entry.value))
            .collect::<Result<Vec<_>, Error>>()?;
        records.sort_by(|left, right| {
            right
                .accepted_lsn
                .cmp(&left.accepted_lsn)
                .then_with(|| left.delivery_id.cmp(&right.delivery_id))
        });
        records.truncate(limit);
        Ok(records)
    }

    pub fn recent_revisions(
        snapshot: &ReadSnapshot<'_>,
        connector_id: &[u8],
        limit: usize,
    ) -> Result<Vec<RevisionRecord>, Error> {
        let limit = bounded_limit(limit)?;
        if connector_id.is_empty() {
            return Err(Error::new(ErrorCode::InvalidArgument));
        }
        let mut records: Vec<RevisionRecord> = snapshot
            .scan_prefix(
                ProjectionId::SourceConnectors,
                &scoped_prefix(REVISION_PREFIX, connector_id),
                MAXIMUM_REGISTRY_SCAN,
            )?
            .into_iter()
            .map(|entry| decode(&entry.value))
            .collect::<Result<Vec<_>, Error>>()?;
        records.sort_by(|left, right| {
            right
                .lsn
                .cmp(&left.lsn)
                .then_with(|| left.source_id.cmp(&right.source_id))
        });
        records.truncate(limit);
        Ok(records)
    }
}

fn bound_mutations(value: &SourceConnectorBound, lsn: u64) -> Result<Vec<Mutation>, Error> {
    let record = ConnectorRecord {
        connector_id: value.connector_id.clone(),
        provider: value.provider.clone(),
        external_account: value.external_account.clone(),
        credential_version: value.credential_version,
        signature_scheme: u8::from(value.signature_scheme),
        scopes: value.scopes.clone(),
        state: u8::from(value.state),
        bound_lsn: lsn,
    };
    Ok(vec![
        Mutation::put(
            scoped_prefix(CONNECTOR_PREFIX, &value.connector_id),
            encode(&record)?,
        ),
        Mutation::put(
            scoped_prefix(NONCE_PREFIX, &value.consent_nonce),
            lsn.to_le_bytes().to_vec(),
        ),
    ])
}

fn accepted_mutations(value: &SourceDeliveryAccepted, lsn: u64) -> Result<Vec<Mutation>, Error> {
    let record = DeliveryRecord {
        connector_id: value.connector_id.clone(),
        delivery_id: value.delivery_id.clone(),
        accepted_lsn: lsn,
        body_digest: value.body_digest.clone(),
        body_bytes: value.body_bytes,
        event_name: value.event_name.clone(),
        attempt: 0,
        state: u8::from(SourceDeliveryState::Accepted),
        next_attempt_at_ns: 0,
        detail: String::new(),
        settled_lsn: 0,
    };
    Ok(vec![Mutation::put(
        delivery_key(&value.connector_id, &value.delivery_id),
        encode(&record)?,
    )])
}

fn settled_mutations(
    snapshot: &ReadSnapshot<'_>,
    value: &SourceDeliverySettled,
    lsn: u64,
) -> Result<Vec<Mutation>, Error> {
    let key = delivery_key(&value.connector_id, &value.delivery_id);
    let Some(bytes) = snapshot.get(ProjectionId::SourceConnectors, &key)? else {
        return Ok(Vec::new());
    };
    let mut record = decode::<DeliveryRecord>(&bytes)?;
    record.attempt = value.attempt;
    record.state = u8::from(value.state);
    record.next_attempt_at_ns = value.next_attempt_at_ns;
    record.detail.clone_from(&value.detail);
    record.settled_lsn = lsn;
    Ok(vec![Mutation::put(key, encode(&record)?)])
}

fn revision_mutations(value: &SourceRevisionObserved, lsn: u64) -> Result<Vec<Mutation>, Error> {
    let record = RevisionRecord {
        connector_id: value.connector_id.clone(),
        source_id: value.source_id.clone(),
        revision: value.revision.clone(),
        content_digest: value.content_digest.clone(),
        observed_at_ns: value.observed_at_ns,
        lsn,
    };
    Ok(vec![Mutation::put(
        revision_key(&value.connector_id, &value.source_id),
        encode(&record)?,
    )])
}

fn bounded_limit(limit: usize) -> Result<usize, Error> {
    if limit == 0 {
        return Err(Error::new(ErrorCode::InvalidArgument));
    }
    Ok(if limit > MAXIMUM_REGISTRY_SCAN {
        MAXIMUM_REGISTRY_SCAN
    } else {
        limit
    })
}

fn scoped_prefix(prefix: u8, identifier: &[u8]) -> Vec<u8> {
    let mut key = Vec::with_capacity(1 + identifier.len());
    key.push(prefix);
    key.extend_from_slice(identifier);
    key
}

fn delivery_key(connector_id: &[u8], delivery_id: &[u8]) -> Vec<u8> {
    let mut key = scoped_prefix(DELIVERY_PREFIX, connector_id);
    key.extend_from_slice(blake3::hash(delivery_id).as_bytes());
    key
}

fn revision_key(connector_id: &[u8], source_id: &str) -> Vec<u8> {
    let mut key = scoped_prefix(REVISION_PREFIX, connector_id);
    key.extend_from_slice(blake3::hash(source_id.as_bytes()).as_bytes());
    key
}
