#![allow(clippy::cast_possible_truncation)]

use anyhow::{Context, Result, ensure};
use hm_core::{ConversationId, LSN};
use hm_schema::event::{self, Boundary, EventHistory};
use hm_schema::events::{Authority, EventEnvelope};
use hm_serve::actor::{ActorEngine, IncomingEvent};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;

pub const FORMAT: &str = "hypermind.events.v1";

#[derive(Serialize, Deserialize)]
struct ExportEvent {
    lsn: u64,
    kind: u8,
    conversation: [u8; 16],
    source_wall_timestamp_ns: i64,
    envelope: EventEnvelope,
}

pub(crate) async fn export_stream(actor: &ActorEngine) -> Result<(Vec<u8>, u64)> {
    let count = actor.stats().await?.log_events;
    let mut body = Vec::new();
    writeln!(
        body,
        "{}",
        json!({"format":FORMAT,"actor":actor.actor().get(),"events":count,"plaintext":true})
    )?;
    let mut cursor = 0;
    while cursor < count {
        let frames = actor.frames_since(LSN::new(cursor), None, 256).await?;
        ensure!(
            !frames.is_empty(),
            "export stopped before the recorded event count"
        );
        for frame in frames {
            let verified = actor.verified_event(frame.header.lsn).await?;
            let row = ExportEvent {
                lsn: frame.header.lsn.get(),
                kind: frame.header.kind as u8,
                conversation: frame.header.conversation.into_bytes(),
                source_wall_timestamp_ns: frame.header.wall_timestamp_ns.get(),
                envelope: verified.envelope,
            };
            writeln!(body, "{}", serde_json::to_string(&row)?)?;
            cursor = row.lsn;
        }
        ensure!(
            body.len() <= crate::archive::MAXIMUM_MEMBER_BYTES,
            "this ledger exports more than the maximum member size; this version cannot archive it and will not truncate the stream"
        );
    }
    Ok((body, count))
}

pub async fn export(path: &Path, output: &Path) -> Result<Value> {
    let _lock = crate::actors::operation_lock(path)?;
    let config = hm_serve::config::load(path)?;
    crate::actors::require_offline(&config)?;
    let actor = crate::actors::open(&config, crate::first_actor(&config)?.actor).await?;
    let result = async {
        let mut file = OpenOptions::new().write(true).create_new(true).mode(0o600).open(output)?;
        let (body, count) = export_stream(&actor).await?;
        file.write_all(&body)?;
        file.sync_all()?;
        File::open(output.parent().context("output directory missing")?)?.sync_all()?;
        Ok(json!({"ok":true,"format":FORMAT,"events":count,"output":output,"plaintext":true,"permissions":"0600"}))
    }.await;
    actor.shutdown().await?;
    result
}

struct History<'a>(&'a [ExportEvent]);
impl EventHistory for History<'_> {
    fn kind_at(&self, lsn: LSN) -> Option<event::EventKind> {
        self.0
            .get(lsn.get().checked_sub(1)? as usize)
            .and_then(|row| event::EventKind::try_from(row.kind).ok())
    }
    fn authority_at(&self, lsn: LSN) -> Option<Authority> {
        self.0
            .get(lsn.get().checked_sub(1)? as usize)
            .map(|row| row.envelope.authority)
    }
}

pub async fn import(path: &Path, input: &Path) -> Result<Value> {
    let bytes = std::fs::read(input)?;
    let source = std::str::from_utf8(&bytes)?;
    let mut lines = source.lines();
    let manifest: Value = serde_json::from_str(lines.next().context("empty import")?)?;
    if manifest["format"] != FORMAT {
        let _lock = crate::actors::operation_lock(path)?;
        let config = hm_serve::config::load(path)?;
        crate::actors::require_offline(&config)?;
        return crate::import::run(&config, input).await;
    }
    let count = manifest["events"]
        .as_u64()
        .context("export count missing")?;
    let mut rows = Vec::<ExportEvent>::new();
    let mut incoming = Vec::new();
    for line in lines {
        let row: ExportEvent = serde_json::from_str(line)?;
        ensure!(row.lsn == rows.len() as u64 + 1, "noncontiguous export LSN");
        let kind = event::EventKind::try_from(row.kind)
            .map_err(|()| anyhow::anyhow!("invalid exported kind"))?;
        let payload = event::encode_event_envelope(&row.envelope);
        event::verify_event_with_history(&payload, kind, Boundary::Import, &History(&rows))?;
        incoming.push(IncomingEvent {
            kind: hm_ledger::frame::EventKind::try_from(row.kind)?,
            conversation: ConversationId::new(row.conversation),
            payload,
        });
        rows.push(row);
    }
    ensure!(
        count == rows.len() as u64,
        "truncated export or incorrect count"
    );
    let _lock = crate::actors::operation_lock(path)?;
    let config = hm_serve::config::load(path)?;
    crate::actors::require_offline(&config)?;
    let actor = crate::actors::open(&config, crate::first_actor(&config)?.actor).await?;
    let result = async {
        let existing = actor.stats().await?.log_events;
        ensure!(existing == 0 || existing == count,"native import requires empty destination or complete identical import");
        if existing == 0 && count != 0 {
            let digest = blake3::hash(&bytes);
            actor.append_idempotent(digest.as_bytes()[..16].try_into()?,1,incoming).await?;
        }
        ensure!(actor.stats().await?.log_events == count,"imported event count differs; no success claimed");
        for row in rows {
            let actual = actor.verified_event(LSN::new(row.lsn)).await?.envelope;
            ensure!(actual.payload == row.envelope.payload && actual.authority == row.envelope.authority && actual.run_id == row.envelope.run_id && actual.event_time_ns == row.envelope.event_time_ns,"native import verification mismatch");
        }
        Ok(json!({"ok":true,"verified":true,"format":FORMAT,"events":count,"resumed":existing > 0,"sealing":"reencrypted_for_destination","wall_timestamps":"new_ledger_ingestion_times"}))
    }.await;
    actor.shutdown().await?;
    result
}
