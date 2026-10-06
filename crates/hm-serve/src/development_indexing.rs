use crate::{
    actor::{ActorEngine, IncomingEvent},
    context_retrieval::{self, EmbeddingProvider, RetrievalError},
};
use hm_context::{ContextError, Scope, digest_bytes, retrieval::SourceKind};
use hm_core::{ConversationId, LSN};
use hm_ledger::frame::EventKind;
use hm_schema::{
    event::{CURRENT_SCHEMA_VERSION, encode_event_envelope},
    events::{EventEnvelope, EventPayload, ProviderFrame, Retention, Sensitivity},
};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::PathBuf};
const PROVIDER: &str = "hypermind/development-indexing/v1";
static WRITER: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FileTarget {
    pub root: PathBuf,
    pub path: PathBuf,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GitTarget {
    pub repository: PathBuf,
    pub object_id: String,
    pub opt_in: bool,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IndexingPolicy {
    pub scope: Scope,
    pub principal: Scope,
    pub revision: u64,
    pub enabled: bool,
    pub files: Vec<FileTarget>,
    pub commits: Vec<GitTarget>,
    pub maximum_sources: usize,
    pub maximum_source_bytes: usize,
    pub maximum_embedding_items: usize,
    pub maximum_embedding_bytes: usize,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
struct Indexed {
    kind: Option<SourceKind>,
    revision: u64,
    digest: String,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
struct Checkpoint {
    cursor: usize,
    frontier: u64,
    earliest_dirty: Option<u64>,
    indexed: BTreeMap<String, Indexed>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
struct Journal {
    version: u32,
    scope: Scope,
    policy: IndexingPolicy,
    checkpoint: Checkpoint,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IndexingReceipt {
    pub policy_revision: u64,
    pub scanned: usize,
    pub changed: usize,
    pub scan_cursor: usize,
    pub earliest_dirty: Option<u64>,
    pub frontier: u64,
    pub vector_checkpoint: Option<String>,
    pub embedded: usize,
    pub remaining: usize,
    pub unavailable: Option<String>,
    pub usage: String,
}
#[derive(Default)]
struct State {
    journal: Option<Journal>,
    tail: u64,
    dirty: Vec<u64>,
    native_sources: BTreeMap<String, Indexed>,
}
fn conversation(scope: &Scope) -> Result<ConversationId, RetrievalError> {
    Ok(ConversationId::derive(&format!(
        "development-indexing:{}",
        scope.digest()?
    )))
}
async fn replay(actor: &ActorEngine, scope: &Scope) -> Result<State, RetrievalError> {
    let frames = actor.frames_since(LSN::new(0), None, 100001).await?;
    if frames.len() > 100000 {
        return Err(ContextError::Capacity.into());
    }
    let mut state = State::default();
    for frame in frames {
        state.tail = frame.header.lsn.get();
        if frame.header.kind != EventKind::ProviderFrame {
            continue;
        }
        let event = actor.verified_event(frame.header.lsn).await?;
        let EventPayload::ProviderFrame(provider) = event.envelope.payload else {
            continue;
        };
        if provider.provider == PROVIDER {
            let journal: Journal = serde_json::from_slice(&provider.api_content)?;
            if journal.version != 1 || frame.header.conversation != conversation(&journal.scope)? {
                return Err(ContextError::Conflict.into());
            }
            if journal.scope == *scope {
                state.journal = Some(journal);
            }
            continue;
        }
        if [
            crate::context_history::HISTORY_PROVIDER,
            crate::context_memory::MEMORY_PROVIDER,
            context_retrieval::RETRIEVAL_PROVIDER,
            "hypermind/context-session/v1",
        ]
        .contains(&provider.provider.as_str())
        {
            let value: serde_json::Value = serde_json::from_slice(&provider.api_content)?;
            if provider.provider == context_retrieval::RETRIEVAL_PROVIDER
                && value["operation"]["operation"] == "source"
            {
                let source: context_retrieval::SourceRecord =
                    serde_json::from_value(value["operation"]["source"].clone())?;
                if source.scope == *scope
                    && (source.id.starts_with("indexed-file:")
                        || source.id.starts_with("indexed-commit:"))
                {
                    let digest = if source.tombstoned {
                        String::new()
                    } else if source.kind == SourceKind::Commit {
                        source
                            .provenance
                            .first()
                            .and_then(|p| p.source_id.strip_prefix("commit:"))
                            .unwrap_or("")
                            .to_string()
                    } else {
                        source.content_digest.clone()
                    };
                    state.native_sources.insert(
                        source.id,
                        Indexed {
                            kind: Some(source.kind),
                            revision: source.revision,
                            digest,
                        },
                    );
                }
            }
            let bound = if provider.provider == "hypermind/context-session/v1" {
                &value["request"]["scope"]
            } else {
                &value["scope"]
            };
            if serde_json::from_value::<Scope>(bound.clone()).ok().as_ref() == Some(scope)
                && (provider.provider != context_retrieval::RETRIEVAL_PROVIDER
                    || value["operation"]["operation"] != "backfill")
            {
                state.dirty.push(frame.header.lsn.get());
            }
        }
    }
    if let Some(journal) = &mut state.journal {
        journal
            .checkpoint
            .indexed
            .extend(state.native_sources.clone());
    }
    Ok(state)
}
async fn append(actor: &ActorEngine, journal: Journal) -> Result<u64, RetrievalError> {
    let _guard = crate::context_jobs::CONTEXT_MUTATIONS.lock().await;
    let tail = actor.stats().await?.applied.last_lsn;
    let content = serde_json::to_vec(&journal)?;
    if content.len() > 512 * 1024 {
        return Err(ContextError::Capacity.into());
    }
    let payload = encode_event_envelope(&EventEnvelope {
        schema_version: CURRENT_SCHEMA_VERSION,
        payload: EventPayload::ProviderFrame(Box::new(ProviderFrame {
            provider: PROVIDER.into(),
            api_content: content,
        })),
        connection_id: None,
        client_seq: 0,
        client_event_index: 0,
        client_event_count: 1,
        origin_actor: 0,
        run_id: None,
        model_provenance: None,
        authority: hm_schema::events::Authority::RuntimeFact,
        retention: Retention::Durable,
        sensitivity: Sensitivity::Personal,
        event_time_ns: 0,
    });
    Ok(actor
        .append_if_tail(
            tail,
            vec![IncomingEvent {
                kind: EventKind::ProviderFrame,
                conversation: conversation(&journal.scope)?,
                payload,
            }],
        )
        .await?
        .last_lsn
        .get())
}
fn validate(policy: &IndexingPolicy) -> Result<(), RetrievalError> {
    policy.scope.validate()?;
    if policy.principal != policy.scope
        || policy.revision == 0
        || policy.files.len() + policy.commits.len() > 128
        || !(1..=16).contains(&policy.maximum_sources)
        || !(1..=128 * 1024).contains(&policy.maximum_source_bytes)
        || !(1..=16).contains(&policy.maximum_embedding_items)
        || !(1..=16 * 128 * 1024).contains(&policy.maximum_embedding_bytes)
    {
        return Err(ContextError::Capacity.into());
    }
    for target in &policy.files {
        let root = std::fs::canonicalize(&target.root)?;
        if !root.is_dir()
            || target.path.is_absolute()
            || target.path.components().any(|c| {
                matches!(
                    c,
                    std::path::Component::ParentDir | std::path::Component::Prefix(_)
                )
            })
        {
            return Err(ContextError::ScopeMismatch.into());
        }
    }
    for target in &policy.commits {
        if !target.opt_in
            || ![40, 64].contains(&target.object_id.len())
            || !target.object_id.bytes().all(|b| b.is_ascii_hexdigit())
            || !std::fs::canonicalize(&target.repository)?.is_dir()
        {
            return Err(ContextError::ScopeMismatch.into());
        }
    }
    Ok(())
}
fn file_id(scope: &Scope, target: &FileTarget) -> Result<String, RetrievalError> {
    Ok(format!(
        "indexed-file:{}",
        digest_bytes(&serde_json::to_vec(&(scope, target))?)
    ))
}
fn commit_id(scope: &Scope, target: &GitTarget) -> Result<String, RetrievalError> {
    Ok(format!(
        "indexed-commit:{}",
        digest_bytes(&serde_json::to_vec(&(
            scope,
            &target.repository,
            &target.object_id
        ))?)
    ))
}
fn target_ids(policy: &IndexingPolicy) -> Result<Vec<String>, RetrievalError> {
    policy
        .files
        .iter()
        .map(|t| file_id(&policy.scope, t))
        .chain(policy.commits.iter().map(|t| commit_id(&policy.scope, t)))
        .collect()
}
pub async fn configure(
    actor: &ActorEngine,
    owner: &Scope,
    caller: &Scope,
    expected_revision: u64,
    policy: IndexingPolicy,
) -> Result<u64, RetrievalError> {
    let _guard = WRITER.lock().await;
    if owner != caller || &policy.scope != owner {
        return Err(ContextError::ScopeMismatch.into());
    }
    validate(&policy)?;
    let state = replay(actor, owner).await?;
    if state.journal.as_ref().map_or(0, |j| j.policy.revision) != expected_revision
        || policy.revision
            != expected_revision
                .checked_add(1)
                .ok_or(ContextError::Capacity)?
    {
        return Err(ContextError::Conflict.into());
    }
    let mut checkpoint = state
        .journal
        .as_ref()
        .map(|j| j.checkpoint.clone())
        .unwrap_or_default();
    let allowed = target_ids(&policy)?;
    for (id, indexed) in &mut checkpoint.indexed {
        if indexed.revision > 0 && (!policy.enabled || !allowed.contains(id)) {
            context_retrieval::tombstone(
                actor,
                owner,
                indexed.kind.ok_or(ContextError::Conflict)?,
                id,
                indexed.revision,
            )
            .await?;
            indexed.revision += 1;
            indexed.digest.clear();
        }
    }
    checkpoint.cursor = 0;
    checkpoint.earliest_dirty = Some(
        checkpoint
            .earliest_dirty
            .unwrap_or(state.tail + 1)
            .min(state.tail + 1),
    );
    append(
        actor,
        Journal {
            version: 1,
            scope: owner.clone(),
            policy,
            checkpoint,
        },
    )
    .await
}
pub async fn inspect(
    actor: &ActorEngine,
    scope: &Scope,
) -> Result<serde_json::Value, RetrievalError> {
    let state = replay(actor, scope).await?;
    Ok(state.journal.map_or(serde_json::Value::Null,|j|serde_json::json!({"policy_revision":j.policy.revision,"enabled":j.policy.enabled,"file_targets":j.policy.files.len(),"git_targets":j.policy.commits.len(),"scan_cursor":j.checkpoint.cursor,"earliest_dirty":j.checkpoint.earliest_dirty,"frontier":j.checkpoint.frontier,"tracked_sources":j.checkpoint.indexed.len()})))
}
pub async fn reconcile(
    actor: &ActorEngine,
    owner: &Scope,
    caller: &Scope,
    provider: Option<&EmbeddingProvider>,
    now_ns: i64,
) -> Result<IndexingReceipt, RetrievalError> {
    let _guard = WRITER.lock().await;
    if owner != caller {
        return Err(ContextError::ScopeMismatch.into());
    }
    let state = replay(actor, owner).await?;
    let mut journal = state.journal.ok_or(ContextError::Unavailable(
        "indexing policy unavailable".into(),
    ))?;
    if !journal.policy.enabled {
        return Err(ContextError::Unavailable("indexing disabled".into()).into());
    }
    validate(&journal.policy)?;
    let policy = &journal.policy;
    let cp = &mut journal.checkpoint;
    let observed = state
        .dirty
        .iter()
        .copied()
        .filter(|lsn| *lsn > cp.frontier)
        .min();
    cp.earliest_dirty = match (cp.earliest_dirty, observed) {
        (Some(a), Some(b)) => Some(a.min(b)),
        (a, b) => a.or(b),
    };
    let total = policy.files.len() + policy.commits.len();
    let mut scanned = 0;
    let mut changed = 0;
    while cp.cursor < total && scanned < policy.maximum_sources {
        let position = cp.cursor;
        if position < policy.files.len() {
            let target = &policy.files[position];
            let id = file_id(owner, target)?;
            let previous = cp.indexed.get(&id).cloned().unwrap_or_default();
            let root = std::fs::canonicalize(&target.root)?;
            let full = root.join(&target.path);
            if !full.exists() {
                if previous.revision > 0 && !previous.digest.is_empty() {
                    context_retrieval::tombstone(
                        actor,
                        owner,
                        SourceKind::File,
                        &id,
                        previous.revision,
                    )
                    .await?;
                    cp.indexed.insert(
                        id,
                        Indexed {
                            kind: Some(SourceKind::File),
                            revision: previous.revision + 1,
                            digest: String::new(),
                        },
                    );
                    changed += 1;
                }
            } else {
                let canonical = std::fs::canonicalize(&full)?;
                if !canonical.starts_with(&root) || !canonical.is_file() {
                    return Err(ContextError::ScopeMismatch.into());
                }
                use std::io::Read;
                let mut bytes = Vec::new();
                std::fs::File::open(&canonical)?
                    .take(policy.maximum_source_bytes as u64 + 1)
                    .read_to_end(&mut bytes)?;
                if bytes.len() > policy.maximum_source_bytes {
                    return Err(ContextError::Capacity.into());
                }
                let digest = digest_bytes(&bytes);
                if previous.digest != digest {
                    context_retrieval::index_file(
                        actor,
                        owner,
                        &context_retrieval::FileIndexRequest {
                            scope: owner.clone(),
                            project_root: root,
                            path: target.path.clone(),
                            id: id.clone(),
                            expected_revision: previous.revision,
                            maximum_bytes: policy.maximum_source_bytes,
                            recorded_at_ns: now_ns,
                        },
                    )
                    .await?;
                    // Recheck observed bytes after the native source admission; a concurrent correction cannot retain its derivative.
                    let mut latest = Vec::new();
                    std::fs::File::open(&canonical)?
                        .take(policy.maximum_source_bytes as u64 + 1)
                        .read_to_end(&mut latest)?;
                    if digest_bytes(&latest) != digest {
                        context_retrieval::tombstone(
                            actor,
                            owner,
                            SourceKind::File,
                            &id,
                            previous.revision + 1,
                        )
                        .await?;
                        return Err(ContextError::Stale.into());
                    }
                    cp.indexed.insert(
                        id,
                        Indexed {
                            kind: Some(SourceKind::File),
                            revision: previous.revision + 1,
                            digest,
                        },
                    );
                    changed += 1;
                }
            }
        } else {
            let target = &policy.commits[position - policy.files.len()];
            let id = commit_id(owner, target)?;
            let previous = cp.indexed.get(&id).cloned().unwrap_or_default();
            if previous.digest.is_empty() {
                context_retrieval::index_commit(
                    actor,
                    owner,
                    &context_retrieval::CommitIndexRequest {
                        scope: owner.clone(),
                        repository: target.repository.clone(),
                        object_id: target.object_id.clone(),
                        id: id.clone(),
                        expected_revision: previous.revision,
                        opt_in: target.opt_in,
                        maximum_bytes: policy.maximum_source_bytes,
                        maximum_millis: 1000,
                        recorded_at_ns: now_ns,
                    },
                )
                .await?;
                cp.indexed.insert(
                    id,
                    Indexed {
                        kind: Some(SourceKind::Commit),
                        revision: previous.revision + 1,
                        digest: target.object_id.clone(),
                    },
                );
                changed += 1;
            }
        }
        cp.cursor += 1;
        scanned += 1;
        // Persist each successful source checkpoint so a later source refusal does not lose revision progress.
        append(
            actor,
            Journal {
                version: 1,
                scope: journal.scope.clone(),
                policy: policy.clone(),
                checkpoint: cp.clone(),
            },
        )
        .await?;
    }
    let scan_complete = cp.cursor == total;
    if scan_complete {
        cp.cursor = 0;
    }
    let observed_sources = replay(actor, owner).await?;
    if let Some(dirty) = observed_sources
        .dirty
        .iter()
        .copied()
        .filter(|lsn| *lsn > cp.frontier)
        .min()
    {
        cp.earliest_dirty = Some(cp.earliest_dirty.map_or(dirty, |old| old.min(dirty)));
    }
    let report = context_retrieval::backfill(
        actor,
        owner,
        provider,
        policy.maximum_embedding_items,
        policy.maximum_embedding_bytes,
        now_ns,
    )
    .await?;
    if scan_complete && report.remaining == 0 && report.unavailable.is_none() {
        cp.frontier = report.ledger_tail;
        cp.earliest_dirty = None;
    }
    let receipt = IndexingReceipt {
        policy_revision: policy.revision,
        scanned,
        changed,
        scan_cursor: cp.cursor,
        earliest_dirty: cp.earliest_dirty,
        frontier: cp.frontier,
        vector_checkpoint: report.checkpoint,
        embedded: report.embedded,
        remaining: report.remaining,
        unavailable: report.unavailable,
        usage: report.usage,
    };
    append(actor, journal).await?;
    Ok(receipt)
}
