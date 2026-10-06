use crate::actor::ActorEngine;
use hm_context::{
    temporal::IdentityRegistry,
    types::{ContextError, Scope, digest_bytes, validate_id},
};
use hm_ledger::frame::EventKind;
use hm_schema::events::EventPayload;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeSet;

#[derive(Debug)]
pub enum ImportError {
    Context(ContextError),
    Ledger(hm_core::Error),
}
impl std::fmt::Display for ImportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Context(error) => write!(f, "{error}"),
            Self::Ledger(error) => write!(f, "{error}"),
        }
    }
}
impl std::error::Error for ImportError {}
impl From<ContextError> for ImportError {
    fn from(e: ContextError) -> Self {
        Self::Context(e)
    }
}
impl From<serde_json::Error> for ImportError {
    fn from(e: serde_json::Error) -> Self {
        Self::Context(e.into())
    }
}
impl From<hm_core::Error> for ImportError {
    fn from(e: hm_core::Error) -> Self {
        Self::Ledger(e)
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImportEntry {
    pub source_id: String,
    pub kind: String,
    pub digest: String,
    pub payload: Value,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImportBundle {
    pub version: u32,
    pub import_id: String,
    pub scope: Scope,
    pub entries: Vec<ImportEntry>,
    pub digest: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub context_sources: Vec<crate::hypermid_context_import::ContextSourceMaterial>,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImportReceipt {
    pub version: u32,
    pub scope: Scope,
    pub import_id: String,
    pub bundle_digest: String,
    pub accepted: usize,
    pub total: usize,
    pub complete: bool,
    pub last_lsn: u64,
}
impl ImportBundle {
    pub fn computed_digest(&self) -> Result<String, ContextError> {
        let mut copy = self.clone();
        copy.digest.clear();
        Ok(digest_bytes(&serde_json::to_vec(&copy)?))
    }
    pub fn validate(&self) -> Result<(), ContextError> {
        self.scope.validate()?;
        validate_id(&self.import_id)?;
        if self.version != 1
            || self.entries.len() > 100_000
            || self.digest != self.computed_digest()?
        {
            return Err(ContextError::Invalid("invalid import bundle".into()));
        }
        let mut ids = BTreeSet::new();
        for entry in &self.entries {
            reject_excluded(&entry.payload)?;
            if entry.source_id.is_empty()
                || !ids.insert(&entry.source_id)
                || entry.digest != digest_bytes(&serde_json::to_vec(&entry.payload)?)
                || serde_json::to_vec(entry)?.len() > 512 * 1024
            {
                return Err(ContextError::Invalid("invalid import entry".into()));
            }
        }
        Ok(())
    }
}
fn connection(key: &str) -> [u8; 16] {
    let hash = blake3::hash(key.as_bytes());
    let mut result = [0; 16];
    result.copy_from_slice(&hash.as_bytes()[..16]);
    result
}
impl From<crate::context_memory::MemoryError> for ImportError {
    fn from(error: crate::context_memory::MemoryError) -> Self {
        match error {
            crate::context_memory::MemoryError::Context(e) => Self::Context(e),
            crate::context_memory::MemoryError::Ledger(e) => Self::Ledger(e),
        }
    }
}
pub async fn import_batch(
    engine: &ActorEngine,
    registry: &IdentityRegistry,
    bundle: &ImportBundle,
    max_entries: usize,
) -> Result<ImportReceipt, ImportError> {
    use crate::context_memory::{self, MemoryCommand, MemoryRequest};
    bundle.validate()?;
    if max_entries == 0 {
        return Err(ContextError::Invalid("zero import batch".into()).into());
    }
    let actor = registry
        .actor_for_scope(&bundle.scope)
        .ok_or(ContextError::ScopeMismatch)?;
    if engine.actor().get() != actor {
        return Err(ContextError::ScopeMismatch.into());
    }
    let _guard = crate::context_jobs::CONTEXT_MUTATIONS.lock().await;
    let shared_key = format!(
        "hypermid-import-v2:{}:{}",
        bundle.scope.digest()?,
        bundle.import_id
    );
    if let Some(checkpoint) = engine
        .latest_checkpoint(shared_key.as_bytes().to_vec())
        .await?
    {
        let previous: ImportReceipt = serde_json::from_slice(&checkpoint.blob)?;
        if previous.version != 2
            || previous.scope != bundle.scope
            || previous.import_id != bundle.import_id
            || previous.bundle_digest != bundle.digest
            || previous.total != bundle.entries.len()
            || previous.accepted > previous.total
            || previous.complete && previous.accepted != previous.total
        {
            return Err(ContextError::Conflict.into());
        }
    }
    const CONTEXT_KINDS: &[&str] = &[
        "source_reference",
        "source_snapshot",
        "summary",
        "projection",
        "policy_revision",
        "cache_generation",
        "reduction",
    ];
    let is_context = !bundle.context_sources.is_empty()
        || bundle
            .entries
            .iter()
            .any(|entry| CONTEXT_KINDS.contains(&entry.kind.as_str()));
    if is_context {
        if bundle
            .entries
            .iter()
            .any(|entry| !CONTEXT_KINDS.contains(&entry.kind.as_str()))
        {
            return Err(ContextError::Invalid("mixed context and knowledge import".into()).into());
        }
        let prepared = crate::hypermid_context_import::prepare(bundle)?;
        let receipt =
            crate::hypermid_context_import::import_batch_locked(engine, &prepared, max_entries)
                .await?;
        persist_receipt(engine, &shared_key, &receipt).await?;
        return Ok(receipt);
    }
    let state = context_memory::rebuild(engine, &bundle.scope).await?;
    let accepted = state.import_progress(&bundle.import_id, &bundle.digest)?;
    if accepted != usize::MAX {
        state.validate_import(&bundle.entries)?;
    }
    let key = format!(
        "hypermid-import-v2:{}:{}",
        bundle.scope.digest()?,
        bundle.import_id
    );
    let total = bundle.entries.len();
    let mut receipt = ImportReceipt {
        version: 2,
        scope: bundle.scope.clone(),
        import_id: bundle.import_id.clone(),
        bundle_digest: bundle.digest.clone(),
        accepted: if accepted == usize::MAX {
            total
        } else {
            accepted
        },
        total,
        complete: accepted == usize::MAX,
        last_lsn: 0,
    };
    let end = receipt.accepted.saturating_add(max_entries).min(total);
    while receipt.accepted < end {
        let index = receipt.accepted;
        let result = context_memory::execute_locked(
            engine,
            &bundle.scope,
            &bundle.scope,
            MemoryRequest {
                version: 1,
                scope: bundle.scope.clone(),
                request_id: format!("import-row:{}:{index}", digest_bytes(key.as_bytes())),
                command: MemoryCommand::ImportRows {
                    import_id: bundle.import_id.clone(),
                    bundle_digest: bundle.digest.clone(),
                    start: index,
                    total,
                    entries: vec![bundle.entries[index].clone()],
                },
            },
        )
        .await?;
        receipt.accepted += 1;
        receipt.last_lsn = result.last_lsn;
    }
    if receipt.accepted == total {
        if total == 0 && accepted == 0 {
            context_memory::execute_locked(
                engine,
                &bundle.scope,
                &bundle.scope,
                MemoryRequest {
                    version: 1,
                    scope: bundle.scope.clone(),
                    request_id: format!("import-empty:{}", digest_bytes(key.as_bytes())),
                    command: MemoryCommand::ImportRows {
                        import_id: bundle.import_id.clone(),
                        bundle_digest: bundle.digest.clone(),
                        start: 0,
                        total: 0,
                        entries: vec![],
                    },
                },
            )
            .await?;
        }
        let committed = context_memory::execute_locked(
            engine,
            &bundle.scope,
            &bundle.scope,
            MemoryRequest {
                version: 1,
                scope: bundle.scope.clone(),
                request_id: format!("import-commit:{}", digest_bytes(key.as_bytes())),
                command: MemoryCommand::CommitImport {
                    import_id: bundle.import_id.clone(),
                    bundle_digest: bundle.digest.clone(),
                },
            },
        )
        .await?;
        receipt.last_lsn = committed.last_lsn;
        receipt.complete = true;
    }
    persist_receipt(engine, &key, &receipt).await?;
    Ok(receipt)
}

async fn persist_receipt(
    engine: &ActorEngine,
    key: &str,
    receipt: &ImportReceipt,
) -> Result<(), ImportError> {
    let bytes = serde_json::to_vec(receipt)?;
    if engine
        .latest_checkpoint(key.as_bytes().to_vec())
        .await?
        .is_none_or(|previous| previous.blob != bytes)
    {
        let id = connection(&format!("{key}:checkpoint"));
        let sequence = engine.next_client_sequence(id).await?;
        engine
            .write_checkpoint(id, sequence, key.as_bytes().to_vec(), bytes)
            .await?;
    }
    Ok(())
}

fn bad() -> ContextError {
    ContextError::Invalid("invalid Hypermid export".into())
}
fn text<'a>(v: &'a Value, k: &str) -> Result<&'a str, ContextError> {
    v.get(k).and_then(Value::as_str).ok_or_else(bad)
}
fn finish(
    id: String,
    scope: Scope,
    entries: Vec<ImportEntry>,
) -> Result<ImportBundle, ImportError> {
    let mut b = ImportBundle {
        context_sources: vec![],
        version: 1,
        import_id: id,
        scope,
        entries,
        digest: String::new(),
    };
    b.digest = b.computed_digest()?;
    b.validate()?;
    Ok(b)
}
pub fn decode_memory_export(bytes: &[u8]) -> Result<ImportBundle, ImportError> {
    if bytes.len() > 64 * 1024 * 1024 {
        return Err(ContextError::Capacity.into());
    }
    let v: Value = match serde_json::from_slice(bytes) {
        Ok(value) => value,
        Err(_) => {
            let lines = std::str::from_utf8(bytes).map_err(|_| bad())?;
            let mut parsed = lines
                .lines()
                .filter(|line| !line.is_empty())
                .map(serde_json::from_str::<Value>);
            let first = parsed.next().ok_or_else(bad)??;
            if first["kind"] != "manifest" {
                return Err(bad().into());
            }
            let mut entries = Vec::new();
            for line in parsed {
                let line = line?;
                if line["kind"] != "entry" {
                    return Err(bad().into());
                }
                entries.push(line["entry"].clone());
            }
            serde_json::json!({"manifest":first["manifest"],"entries":entries})
        }
    };
    let m = &v["manifest"];
    if m["schema_version"].as_u64() != Some(2) {
        return Err(bad().into());
    }
    let rows = v["entries"].as_array().ok_or_else(bad)?;
    if m["item_count"].as_u64() != Some(rows.len() as u64) {
        return Err(bad().into());
    }
    let mut scope_bytes = b"hypermid.memory.scope.v1\0".to_vec();
    for field in ["owner_id", "project_id"] {
        let value = text(&m["scope"], field)?.as_bytes();
        scope_bytes.extend_from_slice(&(value.len() as u64).to_be_bytes());
        scope_bytes.extend_from_slice(value);
    }
    match m["scope"]["workspace_id"].as_str() {
        Some(workspace) => {
            scope_bytes.push(1);
            scope_bytes.extend_from_slice(&(workspace.len() as u64).to_be_bytes());
            scope_bytes.extend_from_slice(workspace.as_bytes());
        }
        None => scope_bytes.push(0),
    }
    let scope_digest = digest_bytes(&scope_bytes);
    if m["record_count"].as_u64()
        != Some(rows.iter().filter(|row| row["kind"] == "record").count() as u64)
    {
        return Err(bad().into());
    }
    let mut stream = b"hypermid.memory.export.v1\0".to_vec();
    let mut entries = Vec::new();
    for row in rows {
        if ![
            "scope",
            "grant",
            "record",
            "revision",
            "episode_detail",
            "smart_note_detail",
            "summary_detail",
            "source",
            "provenance",
            "lineage",
            "verification",
            "mutation",
        ]
        .contains(&text(row, "kind")?)
        {
            return Err(bad().into());
        }
        if row["payload"]
            .get("owner_scope_digest")
            .is_some_and(|value| value.as_str() != Some(&scope_digest))
        {
            return Err(ContextError::ScopeMismatch.into());
        }
        if text(row, "kind")? == "scope" && row["payload"]["scope"] != m["scope"] {
            return Err(ContextError::ScopeMismatch.into());
        }
        let encoded = serde_json::to_vec(row)?;
        stream.extend_from_slice(&(encoded.len() as u64).to_be_bytes());
        stream.extend_from_slice(&encoded);
        entries.push(ImportEntry {
            source_id: text(row, "item_key")?.into(),
            kind: text(row, "kind")?.into(),
            digest: text(row, "item_digest")?.into(),
            payload: row["payload"].clone(),
        });
    }
    if digest_bytes(&stream) != text(m, "stream_digest")? {
        return Err(bad().into());
    }
    finish(
        text(m, "export_id")?.into(),
        serde_json::from_value(m["scope"].clone())?,
        entries,
    )
}
pub fn decode_context_export(bytes: &[u8]) -> Result<ImportBundle, ImportError> {
    if bytes.len() > 64 * 1024 * 1024 {
        return Err(ContextError::Capacity.into());
    }
    let v: Value = serde_json::from_slice(bytes)?;
    let m = &v["manifest"];
    let binding = &v["binding"];
    if m["schema_version"].as_u64() != Some(1)
        || m["scope"] != binding["scope"]
        || m["session_id"] != binding["session_id"]
        || m["cursor"] != binding["cursor"]
        || digest_bytes(&serde_json::to_vec(binding)?) != text(m, "session_binding_digest")?
        || digest_bytes(&serde_json::to_vec(&m["entries"])?) != text(m, "entries_digest")?
    {
        return Err(bad().into());
    }
    let rows = v["records"].as_array().ok_or_else(bad)?;
    let declared = m["entries"].as_array().ok_or_else(bad)?;
    if rows.len() != declared.len() {
        return Err(bad().into());
    }
    let mut entries = Vec::new();
    let mut previous = None;
    let bound: hm_context::types::Cursor = serde_json::from_value(binding["cursor"].clone())?;
    bound.validate()?;
    for (row, d) in rows.iter().zip(declared) {
        let payload = &row["payload"];
        let kind = text(row, "kind")?;
        if ![
            "source_reference",
            "source_snapshot",
            "summary",
            "projection",
            "policy_revision",
            "cache_generation",
            "reduction",
        ]
        .contains(&kind)
        {
            return Err(bad().into());
        }
        if kind == "source_reference" {
            for id in ["item_id", "source_event_id"] {
                validate_id(text(payload, id)?)?;
            }
            let digest = text(payload, "source_digest")?;
            if digest.len() != 64 || !digest.bytes().all(|b| b.is_ascii_hexdigit()) {
                return Err(bad().into());
            }
            let cursor: hm_context::types::Cursor =
                serde_json::from_value(payload["cursor"].clone())?;
            cursor.validate()?;
            if cursor.epoch != bound.epoch
                || cursor.sequence > bound.sequence
                || previous.is_some_and(|p| cursor <= p)
            {
                return Err(bad().into());
            }
            previous = Some(cursor);
        }
        if row["entry_id"] != d["entry_id"]
            || row["kind"] != d["kind"]
            || payload["scope"] != binding["scope"]
            || payload["session_id"] != binding["session_id"]
            || d["byte_length"].as_u64() != Some(serde_json::to_vec(payload)?.len() as u64)
        {
            return Err(bad().into());
        }
        entries.push(ImportEntry {
            source_id: text(row, "entry_id")?.into(),
            kind: text(row, "kind")?.into(),
            digest: text(d, "content_digest")?.into(),
            payload: payload.clone(),
        });
    }
    finish(
        text(m, "manifest_id")?.into(),
        serde_json::from_value(binding["scope"].clone())?,
        entries,
    )
}

pub(crate) fn reject_excluded(v: &Value) -> Result<(), ContextError> {
    match v {
        Value::Object(map) => {
            for (key, value) in map {
                let k = key.to_ascii_lowercase();
                if [
                    "secret",
                    "embedding",
                    "vector",
                    "credential",
                    "api_key",
                    "access_token",
                    "refresh_token",
                    "writer_lease",
                    "live_lease",
                    "transient_queue",
                    "sdk_payload",
                    "fencing_token",
                    "job_claim",
                ]
                .iter()
                .any(|term| k.contains(term))
                {
                    return Err(bad());
                }
                reject_excluded(value)?;
            }
        }
        Value::Array(values) => {
            for value in values {
                reject_excluded(value)?;
            }
        }
        _ => {}
    }
    Ok(())
}

pub async fn import_receipt(
    engine: &ActorEngine,
    scope: &Scope,
    import_id: &str,
) -> Result<Option<ImportReceipt>, ImportError> {
    validate_id(import_id)?;
    let key = format!("hypermid-import-v2:{}:{}", scope.digest()?, import_id);
    match engine.latest_checkpoint(key.into_bytes()).await? {
        Some(checkpoint) => {
            let receipt: ImportReceipt = serde_json::from_slice(&checkpoint.blob)?;
            if receipt.version != 2
                || &receipt.scope != scope
                || receipt.import_id != import_id
                || receipt.accepted > receipt.total
                || receipt.complete && receipt.accepted != receipt.total
            {
                return Err(ContextError::Conflict.into());
            }
            Ok(Some(receipt))
        }
        None => Ok(None),
    }
}

pub async fn list_import_receipts(
    engine: &ActorEngine,
    scope: &Scope,
    limit: usize,
) -> Result<Vec<ImportReceipt>, ImportError> {
    if limit == 0 || limit > 256 {
        return Err(ContextError::Invalid("invalid receipt limit".into()).into());
    }
    let prefix = format!("hypermid-import-v2:{}:", scope.digest()?);
    let total = engine.stats().await?.log_events;
    let mut ids = BTreeSet::new();
    let mut since = total.saturating_sub(4096);
    loop {
        let frames = engine
            .frames_since(hm_core::LSN::new(since), None, 256)
            .await?;
        if frames.is_empty() {
            break;
        }
        for frame in &frames {
            since = frame.header.lsn.get();
            if frame.header.kind != EventKind::Checkpoint {
                continue;
            }
            let event = engine.verified_event(frame.header.lsn).await?;
            if let EventPayload::Checkpoint(checkpoint) = event.envelope.payload {
                let cursor = &checkpoint.cursor;
                if cursor.len() < 2 {
                    continue;
                }
                let length = u16::from_le_bytes([cursor[0], cursor[1]]) as usize;
                let Some(key) = cursor.get(2..2 + length) else {
                    continue;
                };
                let Ok(key) = std::str::from_utf8(key) else {
                    continue;
                };
                if let Some(id) = key.strip_prefix(&prefix) {
                    ids.insert(id.to_owned());
                }
            }
        }
        if frames.len() < 256 {
            break;
        }
    }
    let mut result = Vec::new();
    for id in ids {
        if let Some(receipt) = import_receipt(engine, scope, &id).await? {
            result.push(receipt);
        }
    }
    result.sort_by(|a, b| b.last_lsn.cmp(&a.last_lsn));
    result.truncate(limit);
    Ok(result)
}
