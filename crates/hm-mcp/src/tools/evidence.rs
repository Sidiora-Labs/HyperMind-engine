#![allow(clippy::missing_errors_doc)]

use super::inspect::InspectHistory;
use crate::Envelope;
use hm_core::{Error, ErrorCode, LSN};
use hm_schema::event::{self, Boundary};
use hm_schema::events::{AttestationDisposition, EventPayload};
use hm_serve::actor::ActorEngine;
use serde_json::json;

pub(crate) const MAXIMUM_EVIDENCE_FRAMES: usize = 65_536;
pub(crate) const MAXIMUM_EVIDENCE_ANSWERS: usize = 128;

pub(crate) async fn run(actor: &ActorEngine, lsn: LSN) -> Result<Envelope, Error> {
    let frames = actor
        .frames_since(LSN::new(0), None, MAXIMUM_EVIDENCE_FRAMES + 1)
        .await?;
    if frames.len() > MAXIMUM_EVIDENCE_FRAMES {
        return Err(Error::new(ErrorCode::CapacityExceeded));
    }
    let mut history = InspectHistory::default();
    let mut used = 0_u64;
    let mut ignored = 0_u64;
    let mut helpful = 0_u64;
    let mut harmful = 0_u64;
    let mut answers = Vec::new();
    let mut truncated = false;
    for frame in &frames {
        let kind = event::EventKind::try_from(frame.header.kind as u8)
            .map_err(|()| Error::new(ErrorCode::InvalidKind).at_lsn(frame.header.lsn))?;
        let verified = event::verify_event_with_history(
            &frame.sealed_payload,
            kind,
            Boundary::Disk,
            &history,
        )?;
        history.record(frame.header.lsn, kind, verified.envelope.authority);
        let EventPayload::Attestation(attestation) = &verified.envelope.payload else {
            continue;
        };
        if attestation.target_lsn != lsn.get() {
            continue;
        }
        match attestation.disposition {
            AttestationDisposition::Used => used += 1,
            AttestationDisposition::Ignored => ignored += 1,
            AttestationDisposition::Helpful => helpful += 1,
            AttestationDisposition::Harmful => harmful += 1,
        }
        if answers.len() >= MAXIMUM_EVIDENCE_ANSWERS {
            truncated = true;
            continue;
        }
        answers.push((
            frame.header.lsn.get(),
            frame.header.conversation.to_string(),
            disposition_name(attestation.disposition),
        ));
    }

    let mut envelope = Envelope::empty();
    envelope
        .provenance
        .push(format!("hm://{}/lsn/{}", actor.actor(), lsn.get()));
    for (attestation_lsn, _, _) in &answers {
        envelope
            .provenance
            .push(format!("hm://{}/lsn/{attestation_lsn}", actor.actor()));
    }
    envelope.items.push(json!({
        "surface": "evidence",
        "lsn": lsn.get(),
        "counts_from": "ledger_scan",
        "attestations": {
            "used": used,
            "ignored": ignored,
            "helpful": helpful,
            "harmful": harmful,
        },
        "answers": answers
            .iter()
            .map(|(attestation_lsn, conversation, disposition)| json!({
                "attestation_lsn": attestation_lsn,
                "conversation": conversation,
                "disposition": disposition,
            }))
            .collect::<Vec<_>>(),
        "truncated": truncated,
    }));
    Ok(envelope)
}

pub(crate) const fn disposition_name(value: AttestationDisposition) -> &'static str {
    match value {
        AttestationDisposition::Used => "used",
        AttestationDisposition::Ignored => "ignored",
        AttestationDisposition::Helpful => "helpful",
        AttestationDisposition::Harmful => "harmful",
    }
}
