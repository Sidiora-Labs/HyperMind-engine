use crate::{
    actor::ActorEngine,
    context_memory::{
        self, MEMORY_PROVIDER, MemoryCommand, MemoryError, MemoryRequest, RecordStatus,
    },
};
use hm_context::{ContextError, Scope, digest_bytes};
use hm_core::{Error, ErrorCode, LSN};
use hm_schema::{event::encode_event_envelope, events::EventPayload};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RetentionPolicy {
    pub revision: u64,
    pub tombstone_grace_ns: i64,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RetainedCopy {
    pub digest_kind: String,
    pub lsn: u64,
    pub kind: String,
    pub id: String,
    pub digest: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RetentionPlan {
    pub version: u32,
    pub scope: Scope,
    pub record_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub source_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub source_copies: Vec<RetainedCopy>,
    pub expected_tail: u64,
    pub planned_at_ns: i64,
    pub policy: RetentionPolicy,
    pub affected_lsns: Vec<u64>,
    pub original_root: Vec<u8>,
    pub storage_semantics: String,
    pub external_copies: String,
    pub digest: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RetentionApproval {
    pub plan: RetentionPlan,
    pub reviewer: Scope,
    pub plan_digest: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PurgeReceipt {
    pub version: u32,
    pub scope: Scope,
    pub plan_digest: String,
    pub record_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub source_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub source_copies: Vec<RetainedCopy>,
    pub original_root: Vec<u8>,
    pub redacted_frames: usize,
    pub managed_cleanup_complete: bool,
    pub storage_semantics: String,
    pub external_copies: String,
    pub replayed: bool,
}
pub(crate) struct ValidatedPurge {
    pub expected_tail: u64,
    pub root: [u8; 32],
    pub plan: [u8; 32],
    pub replacements: BTreeMap<u64, Vec<u8>>,
}
fn unavailable(reason: &str) -> MemoryError {
    ContextError::Unavailable(reason.into()).into()
}
fn now() -> Result<i64, MemoryError> {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|_| unavailable("runtime clock"))?
        .as_nanos();
    i64::try_from(nanos).map_err(|_| unavailable("runtime clock range"))
}
fn plan_digest(plan: &RetentionPlan) -> Result<String, MemoryError> {
    let mut blank = plan.clone();
    blank.digest.clear();
    Ok(digest_bytes(&serde_json::to_vec(&blank)?))
}
fn digest_array(hex: &str) -> Result<[u8; 32], MemoryError> {
    if hex.len() != 64 {
        return Err(ContextError::Invalid("plan digest".into()).into());
    }
    let mut out = [0; 32];
    for (i, byte) in out.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16)
            .map_err(|_| ContextError::Invalid("plan digest".into()))?;
    }
    Ok(out)
}
pub async fn plan(
    actor: &ActorEngine,
    principal: &Scope,
    scope: &Scope,
    record_ids: Vec<String>,
    expected_tail: u64,
    policy: RetentionPolicy,
) -> Result<RetentionPlan, MemoryError> {
    let _guard = crate::context_jobs::CONTEXT_MUTATIONS.lock().await;
    build(
        actor,
        principal,
        scope,
        record_ids,
        expected_tail,
        policy,
        now()?,
    )
    .await
    .map(|(p, _)| p)
}
async fn build(
    actor: &ActorEngine,
    principal: &Scope,
    scope: &Scope,
    mut record_ids: Vec<String>,
    expected_tail: u64,
    policy: RetentionPolicy,
    planned_at_ns: i64,
) -> Result<(RetentionPlan, BTreeMap<u64, Vec<u8>>), MemoryError> {
    scope.validate()?;
    if principal != scope {
        return Err(ContextError::ScopeMismatch.into());
    }
    if policy.revision == 0
        || policy.tombstone_grace_ns < 0
        || record_ids.is_empty()
        || record_ids.len() > 32
    {
        return Err(ContextError::Invalid("retention bounds".into()).into());
    }
    record_ids.sort();
    record_ids.dedup();
    let tail = actor.stats().await?.applied.last_lsn;
    if tail.get() != expected_tail {
        return Err(ContextError::Stale.into());
    }
    if tail.get() > 10000 {
        return Err(unavailable("bounded retention scan exceeded"));
    }
    let state = context_memory::rebuild(actor, scope).await?;
    let targets: BTreeSet<_> = record_ids.iter().cloned().collect();
    let mut revisions = BTreeMap::new();
    let mut source_ids = BTreeSet::new();
    for id in &record_ids {
        let record = state
            .records
            .get(id)
            .ok_or_else(|| unavailable("native record"))?;
        if record.status != RecordStatus::Tombstoned
            || record.pinned
            || record
                .retention_until_ns
                .is_some_and(|until| until > planned_at_ns)
        {
            return Err(unavailable("protected record or retention window"));
        }
        let history: Vec<_> = state.revisions.values().filter(|r| r.id == *id).collect();
        if history
            .iter()
            .any(|r| r.pinned || !r.lineage.is_empty() || !r.contradictions.is_empty())
        {
            return Err(unavailable(
                "pinned or required evidence dependency unsupported",
            ));
        }
        for revision in &history {
            for provenance in &revision.provenance {
                source_ids.insert(provenance.source_id.clone());
            }
        }
        revisions.insert(
            id.clone(),
            history
                .iter()
                .map(|r| r.revision_digest.clone())
                .collect::<Vec<_>>(),
        );
    }
    if state.revisions.values().any(|r| {
        !targets.contains(&r.id)
            && (r
                .lineage
                .iter()
                .any(|l| targets.contains(&l.parent_record_id))
                || r.contradictions.iter().any(|id| targets.contains(id)))
    }) {
        return Err(unavailable("retained lineage or competing evidence"));
    }
    for source_id in &source_ids {
        let source = state
            .sources
            .get(source_id)
            .ok_or_else(|| unavailable("missing original source"))?;
        if !source.tombstoned {
            return Err(unavailable("original source is not withdrawn"));
        }
        if state.revisions.values().any(|r| {
            !targets.contains(&r.id) && r.provenance.iter().any(|p| &p.source_id == source_id)
        }) {
            return Err(unavailable(
                "original source is protected by an unrelated historical citation",
            ));
        }
    }
    let frames = if tail.get() == 0 {
        vec![]
    } else {
        actor.frames_since(LSN::new(0), None, 10000).await?
    };
    if frames.len() as u64 != tail.get() {
        return Err(unavailable("incomplete retention scan"));
    }
    let mut replacements = BTreeMap::new();
    let mut tombstones = BTreeMap::new();
    let mut source_tombstones = BTreeMap::new();
    let mut source_copies = Vec::new();
    for frame in frames {
        if frame.header.kind != hm_ledger::frame::EventKind::ProviderFrame {
            return Err(unavailable("non-native ledger copies unsupported"));
        }
        let mut envelope = actor.verified_event(frame.header.lsn).await?.envelope;
        let EventPayload::ProviderFrame(provider) = &mut envelope.payload else {
            return Err(unavailable("non-native envelope"));
        };
        if provider.provider != MEMORY_PROVIDER {
            return Err(unavailable(
                "history/cache/derived provider copies unsupported",
            ));
        }
        let mut event: serde_json::Value = serde_json::from_slice(&provider.api_content)?;
        let event_scope: Scope = serde_json::from_value(event["scope"].clone())?;
        let command: MemoryCommand = serde_json::from_value(event["command"].clone())?;
        let (id, is_source) = match &command {
            MemoryCommand::Create { record } | MemoryCommand::Revise { record, .. } => {
                (record.id.clone(), false)
            }
            MemoryCommand::Tombstone { id, .. }
            | MemoryCommand::SetStatus { id, .. }
            | MemoryCommand::PurgedRecord { id, .. } => (id.clone(), false),
            MemoryCommand::Source { source } => (source.id.clone(), true),
            MemoryCommand::TombstoneSource { id } | MemoryCommand::PurgedSource { id, .. } => {
                (id.clone(), true)
            }
            _ => return Err(unavailable("mixed/import/worker/grant copies unsupported")),
        };
        if event_scope != *scope
            || if is_source {
                !source_ids.contains(&id)
            } else {
                !targets.contains(&id)
            }
        {
            continue;
        }
        if matches!(
            command,
            MemoryCommand::Tombstone { .. }
                | MemoryCommand::SetStatus {
                    status: RecordStatus::Tombstoned,
                    ..
                }
        ) {
            tombstones.insert(id.clone(), frame.header.wall_timestamp_ns.get());
        }
        if matches!(command, MemoryCommand::TombstoneSource { .. }) {
            source_tombstones.insert(id.clone(), frame.header.wall_timestamp_ns.get());
        }
        let original_digest = event["digest"]
            .as_str()
            .ok_or_else(|| unavailable("native receipt digest"))?
            .to_owned();
        let sanitized = if is_source {
            let source_digest = state.sources[&id].digest.clone();
            let copy_digest = match &command {
                MemoryCommand::Source { source } => source.digest.clone(),
                _ => source_digest.clone(),
            };
            source_copies.push(RetainedCopy {
                digest_kind: "source_content_sha256".into(),
                lsn: frame.header.lsn.get(),
                kind: if matches!(command, MemoryCommand::Source { .. }) {
                    "raw_source"
                } else {
                    "source_withdrawal"
                }
                .into(),
                id: id.clone(),
                digest: copy_digest,
            });
            MemoryCommand::PurgedSource {
                id: id.clone(),
                source_digest,
                original_request_digest: original_digest,
            }
        } else {
            if !source_ids.is_empty() {
                source_copies.push(RetainedCopy {
                    digest_kind: "record_revision_sha256".into(),
                    lsn: frame.header.lsn.get(),
                    kind: if matches!(
                        command,
                        MemoryCommand::Create { .. } | MemoryCommand::Revise { .. }
                    ) {
                        "record_revision"
                    } else {
                        "record_status"
                    }
                    .into(),
                    id: id.clone(),
                    digest: match &command {
                        MemoryCommand::Create { record } | MemoryCommand::Revise { record, .. } => {
                            state
                                .revisions
                                .get(&(id.clone(), record.revision))
                                .ok_or_else(|| unavailable("historical revision inventory"))?
                                .revision_digest
                                .clone()
                        }
                        MemoryCommand::Tombstone {
                            expected_revision, ..
                        }
                        | MemoryCommand::SetStatus {
                            expected_revision, ..
                        } => {
                            let revision = expected_revision
                                .checked_add(1)
                                .ok_or(ContextError::Capacity)?;
                            state
                                .revisions
                                .get(&(id.clone(), revision))
                                .ok_or_else(|| unavailable("historical status revision inventory"))?
                                .revision_digest
                                .clone()
                        }
                        _ => return Err(unavailable("historical revision inventory")),
                    },
                });
            }
            MemoryCommand::PurgedRecord {
                id: id.clone(),
                revision_digests: revisions[&id].clone(),
                original_request_digest: original_digest,
            }
        };
        let request = MemoryRequest {
            version: 1,
            scope: scope.clone(),
            request_id: event["request_id"]
                .as_str()
                .ok_or_else(|| unavailable("native request identity"))?
                .to_owned(),
            command: sanitized.clone(),
        };
        event["command"] = serde_json::to_value(sanitized)?;
        event["digest"] = serde_json::Value::String(digest_bytes(&serde_json::to_vec(&request)?));
        provider.api_content = serde_json::to_vec(&event)?;
        replacements.insert(frame.header.lsn.get(), encode_event_envelope(&envelope));
    }
    for id in &record_ids {
        let timestamp = *tombstones
            .get(id)
            .ok_or_else(|| unavailable("tombstone evidence"))?;
        if timestamp
            .checked_add(policy.tombstone_grace_ns)
            .is_none_or(|until| until > planned_at_ns)
        {
            return Err(unavailable("tombstone grace window"));
        }
    }
    for id in &source_ids {
        let timestamp = *source_tombstones
            .get(id)
            .ok_or_else(|| unavailable("source withdrawal evidence"))?;
        if timestamp
            .checked_add(policy.tombstone_grace_ns)
            .is_none_or(|until| until > planned_at_ns)
        {
            return Err(unavailable("source withdrawal grace window"));
        }
    }
    let root = actor.verification_status().await?.root;
    crate::context_projection::validate_tail(actor, tail).await?;
    let mut plan=RetentionPlan{version:1,scope:scope.clone(),record_ids,source_ids:source_ids.into_iter().collect(),source_copies,expected_tail,planned_at_ns,policy,affected_lsns:replacements.keys().copied().collect(),original_root:root.to_vec(),storage_semantics:"managed_filesystem_removal; original MMR commitments retained; no crypto/device erase".into(),external_copies:"external exports, backups, snapshots and copied ciphertext are not controlled and may remain decryptable".into(),digest:String::new()};
    plan.digest = plan_digest(&plan)?;
    Ok((plan, replacements))
}
pub fn approve(principal: &Scope, plan: RetentionPlan) -> Result<RetentionApproval, MemoryError> {
    if principal != &plan.scope {
        return Err(ContextError::ScopeMismatch.into());
    }
    if plan.digest != plan_digest(&plan)? {
        return Err(ContextError::Conflict.into());
    }
    Ok(RetentionApproval {
        reviewer: principal.clone(),
        plan_digest: plan.digest.clone(),
        plan,
    })
}
pub async fn execute(
    actor: &ActorEngine,
    principal: &Scope,
    approval: RetentionApproval,
) -> Result<PurgeReceipt, MemoryError> {
    let _guard = crate::context_jobs::CONTEXT_MUTATIONS.lock().await;
    let p = &approval.plan;
    if principal != &p.scope || approval.reviewer != *principal {
        return Err(ContextError::ScopeMismatch.into());
    }
    if approval.plan_digest != p.digest || p.digest != plan_digest(p)? {
        return Err(ContextError::Conflict.into());
    }
    let plan_hash = digest_array(&p.digest)?;
    if let Some(storage) = actor.retention_status().await? {
        if storage.plan == plan_hash {
            return Ok(receipt(p, p.affected_lsns.len(), true));
        }
    }
    let runtime = now()?;
    if runtime < p.planned_at_ns || runtime - p.planned_at_ns > 600_000_000_000 {
        return Err(ContextError::Stale.into());
    }
    let (fresh, replacements) = build(
        actor,
        principal,
        &p.scope,
        p.record_ids.clone(),
        p.expected_tail,
        p.policy.clone(),
        p.planned_at_ns,
    )
    .await?;
    if fresh != *p {
        return Err(ContextError::Stale.into());
    }
    let root = p
        .original_root
        .as_slice()
        .try_into()
        .map_err(|_| Error::new(ErrorCode::InvalidArgument))?;
    let storage = actor
        .purge_native(ValidatedPurge {
            expected_tail: p.expected_tail,
            root,
            plan: plan_hash,
            replacements,
        })
        .await?;
    Ok(receipt(p, storage.redacted_frames, false))
}
fn receipt(plan: &RetentionPlan, count: usize, replayed: bool) -> PurgeReceipt {
    PurgeReceipt {
        version: 1,
        scope: plan.scope.clone(),
        plan_digest: plan.digest.clone(),
        record_ids: plan.record_ids.clone(),
        source_ids: plan.source_ids.clone(),
        source_copies: plan.source_copies.clone(),
        original_root: plan.original_root.clone(),
        redacted_frames: count,
        managed_cleanup_complete: true,
        storage_semantics: plan.storage_semantics.clone(),
        external_copies: plan.external_copies.clone(),
        replayed,
    }
}
