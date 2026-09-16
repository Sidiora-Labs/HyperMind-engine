#![allow(clippy::missing_errors_doc)]

use crate::actor::{ActorEngine, AppendOutcome, IncomingEvent};
use hm_core::{ConversationId, Error, ErrorCode, LSN};
use hm_ledger::frame::EventKind;
use hm_ledger::idempotency::ConnectionId;
use hm_schema::event::{CURRENT_SCHEMA_VERSION, encode_event_envelope};
use hm_schema::events::{
    Attestation, AttestationDisposition, Authority, EventEnvelope, EventPayload, Retention,
    Sensitivity,
};
use hm_schema::wire::Attest;
use std::collections::BTreeMap;

pub async fn write(
    actor: &ActorEngine,
    connection_id: ConnectionId,
    request: Attest,
) -> Result<AppendOutcome, Error> {
    if request.client_seq == 0 {
        return Err(Error::new(ErrorCode::InvalidArgument));
    }
    let mut targets = BTreeMap::new();
    add_targets(
        &mut targets,
        request.used.as_deref().unwrap_or_default(),
        AttestationDisposition::Used,
    )?;
    add_targets(
        &mut targets,
        request.ignored.as_deref().unwrap_or_default(),
        AttestationDisposition::Ignored,
    )?;
    add_targets(
        &mut targets,
        request.helpful.as_deref().unwrap_or_default(),
        AttestationDisposition::Helpful,
    )?;
    add_targets(
        &mut targets,
        request.harmful.as_deref().unwrap_or_default(),
        AttestationDisposition::Harmful,
    )?;
    if targets.is_empty() || targets.len() > hm_schema::protocol::MAXIMUM_BATCH_EVENTS {
        return Err(Error::new(ErrorCode::InvalidArgument));
    }
    for target in targets.keys() {
        let found = actor
            .frames_since(LSN::new(target.saturating_sub(1)), None, 1)
            .await?
            .first()
            .is_some_and(|frame| frame.header.lsn.get() == *target);
        if !found {
            return Err(Error::new(ErrorCode::InvalidArgument).at_lsn(LSN::new(*target)));
        }
    }
    let conversation = ConversationId::derive("hypermind.attest");
    let events = targets
        .into_iter()
        .map(|(target_lsn, disposition)| IncomingEvent {
            kind: EventKind::Attestation,
            conversation,
            payload: encode_event_envelope(&EventEnvelope {
                schema_version: CURRENT_SCHEMA_VERSION,
                payload: EventPayload::Attestation(Box::new(Attestation {
                    target_lsn,
                    disposition,
                })),
                connection_id: None,
                client_seq: 0,
                client_event_index: 0,
                client_event_count: 0,
                origin_actor: actor.actor().get(),
                run_id: None,
                model_provenance: None,
                authority: Authority::RuntimeFact,
                retention: Retention::Daily,
                sensitivity: Sensitivity::Personal,
                event_time_ns: 0,
            }),
        })
        .collect();
    actor
        .append_idempotent(connection_id, request.client_seq, events)
        .await
}

fn add_targets(
    targets: &mut BTreeMap<u64, AttestationDisposition>,
    values: &[u64],
    disposition: AttestationDisposition,
) -> Result<(), Error> {
    for value in values {
        if *value == 0 || targets.insert(*value, disposition).is_some() {
            return Err(Error::new(ErrorCode::InvalidArgument));
        }
    }
    Ok(())
}
