use super::*;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    io::Read,
    path::PathBuf,
    process::{Command, Stdio},
    time::{Duration, Instant},
};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FileIndexRequest {
    pub scope: Scope,
    pub project_root: PathBuf,
    pub path: PathBuf,
    pub id: String,
    pub expected_revision: u64,
    pub maximum_bytes: usize,
    #[serde(with = "hm_context::types::timestamp_wire")]
    pub recorded_at_ns: i64,
}

pub async fn index_file(
    actor: &ActorEngine,
    trusted: &Scope,
    request: &FileIndexRequest,
) -> Result<u64, RetrievalError> {
    if &request.scope != trusted {
        return Err(ContextError::ScopeMismatch.into());
    }
    if request.maximum_bytes == 0 || request.maximum_bytes > MAX_TEXT {
        return Err(ContextError::Capacity.into());
    }
    let root = std::fs::canonicalize(&request.project_root)?;
    if !root.is_dir() {
        return Err(ContextError::Invalid("project root is not a directory".into()).into());
    }
    let path = std::fs::canonicalize(root.join(&request.path))?;
    if !path.starts_with(&root) || !path.is_file() {
        return Err(ContextError::ScopeMismatch.into());
    }
    let mut bytes = Vec::new();
    std::fs::File::open(&path)?
        .take(request.maximum_bytes as u64 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > request.maximum_bytes {
        return Err(ContextError::Capacity.into());
    }
    let text = String::from_utf8(bytes.clone())
        .map_err(|_| ContextError::Invalid("file is not UTF-8 text".into()))?;
    let digest = digest_bytes(&bytes);
    let source = SourceRecord {
        scope: trusted.clone(),
        kind: SourceKind::File,
        id: request.id.clone(),
        revision: request
            .expected_revision
            .checked_add(1)
            .ok_or(ContextError::Capacity)?,
        text,
        content_digest: digest.clone(),
        authority: Authority::ExternalObserved,
        provenance: vec![SourceSpan {
            source_id: format!("file:{}", digest_bytes(path.to_string_lossy().as_bytes())),
            source_digest: digest.clone(),
            byte_start: 0,
            byte_end: bytes.len() as u64,
        }],
        occurred_at_ns: None,
        recorded_at_ns: request.recorded_at_ns,
        expires_at_ns: None,
        tombstoned: false,
    };
    let _guard = crate::context_jobs::CONTEXT_MUTATIONS.lock().await;
    if std::fs::canonicalize(root.join(&request.path))? != path {
        return Err(ContextError::Stale.into());
    }
    let mut current = Vec::new();
    std::fs::File::open(&path)?
        .take(request.maximum_bytes as u64 + 1)
        .read_to_end(&mut current)?;
    if digest_bytes(&current) != digest {
        return Err(ContextError::Stale.into());
    }
    source.validate()?;
    let state = state(actor, trusted).await?;
    if state
        .sources
        .get(&(source.kind, source.id.clone()))
        .map_or(0, |s| s.revision)
        != request.expected_revision
    {
        return Err(ContextError::Conflict.into());
    }
    append(
        actor,
        trusted,
        state.tail,
        Operation::Source {
            expected_revision: request.expected_revision,
            source,
        },
    )
    .await
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CommitIndexRequest {
    pub scope: Scope,
    pub repository: PathBuf,
    pub object_id: String,
    pub id: String,
    pub expected_revision: u64,
    pub opt_in: bool,
    pub maximum_bytes: usize,
    pub maximum_millis: u64,
    #[serde(with = "hm_context::types::timestamp_wire")]
    pub recorded_at_ns: i64,
}

pub async fn index_commit(
    actor: &ActorEngine,
    trusted: &Scope,
    request: &CommitIndexRequest,
) -> Result<u64, RetrievalError> {
    if &request.scope != trusted || !request.opt_in {
        return Err(ContextError::ScopeMismatch.into());
    }
    if request.maximum_bytes == 0
        || request.maximum_bytes > MAX_TEXT
        || request.maximum_millis == 0
        || request.maximum_millis > 30_000
    {
        return Err(ContextError::Capacity.into());
    }
    if ![40, 64].contains(&request.object_id.len())
        || !request.object_id.bytes().all(|b| b.is_ascii_hexdigit())
    {
        return Err(ContextError::Invalid("commit object must be a full hash".into()).into());
    }
    let root = std::fs::canonicalize(&request.repository)?;
    let mut child = Command::new("git")
        .arg("-C")
        .arg(&root)
        .args([
            "show",
            "--no-color",
            "--no-ext-diff",
            "--no-textconv",
            "--format=fuller",
            "--stat",
            &request.object_id,
            "--",
        ])
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()?;
    let stdout = child
        .stdout
        .take()
        .ok_or(ContextError::Unavailable("git stdout unavailable".into()))?;
    let max = request.maximum_bytes;
    let reader = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        let result = stdout.take(max as u64 + 1).read_to_end(&mut bytes);
        (result, bytes)
    });
    let deadline = Instant::now() + Duration::from_millis(request.maximum_millis);
    let success = loop {
        if let Some(status) = child.try_wait()? {
            break status.success();
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            break false;
        }
        std::thread::sleep(Duration::from_millis(5));
    };
    let (read, bytes) = reader
        .join()
        .map_err(|_| ContextError::Unavailable("git reader failed".into()))?;
    read?;
    if !success {
        return Err(
            ContextError::Unavailable("git commit read failed or exceeded bounds".into()).into(),
        );
    }
    if bytes.len() > max {
        return Err(ContextError::Capacity.into());
    }
    let text = String::from_utf8(bytes)
        .map_err(|_| ContextError::Invalid("commit is not UTF-8 text".into()))?;
    if text.lines().next()
        != Some(format!("commit {}", request.object_id.to_ascii_lowercase()).as_str())
    {
        return Err(
            ContextError::Invalid("source object is not the requested commit".into()).into(),
        );
    }
    let digest = digest_bytes(text.as_bytes());
    let len = text.len() as u64;
    register_source(
        actor,
        trusted,
        request.expected_revision,
        SourceRecord {
            scope: trusted.clone(),
            kind: SourceKind::Commit,
            id: request.id.clone(),
            revision: request
                .expected_revision
                .checked_add(1)
                .ok_or(ContextError::Capacity)?,
            text,
            content_digest: digest.clone(),
            authority: Authority::ExternalObserved,
            provenance: vec![SourceSpan {
                source_id: format!("commit:{}", request.object_id.to_ascii_lowercase()),
                source_digest: digest,
                byte_start: 0,
                byte_end: len,
            }],
            occurred_at_ns: None,
            recorded_at_ns: request.recorded_at_ns,
            expires_at_ns: None,
            tombstoned: false,
        },
    )
    .await
}

pub(super) async fn add_history(
    actor: &ActorEngine,
    scope: &Scope,
    sources: &mut BTreeMap<Key, SourceRecord>,
) -> Result<(), RetrievalError> {
    let frames = actor
        .frames_since(LSN::new(0), None, MAX_FRAMES + 1)
        .await?;
    if frames.len() > MAX_FRAMES {
        return Err(ContextError::Capacity.into());
    }
    let mut sessions = BTreeSet::new();
    for frame in frames {
        if frame.header.kind != EventKind::ProviderFrame {
            continue;
        }
        let event = actor.verified_event(frame.header.lsn).await?;
        let EventPayload::ProviderFrame(p) = event.envelope.payload else {
            continue;
        };
        if p.provider != crate::context_history::HISTORY_PROVIDER
            && p.provider != "hypermind/context-session/v1"
        {
            continue;
        }
        let value: serde_json::Value = serde_json::from_slice(&p.api_content)?;
        let binding = if p.provider == crate::context_history::HISTORY_PROVIDER {
            &value
        } else {
            &value["request"]
        };
        let bound: Scope = serde_json::from_value(binding["scope"].clone())?;
        if bound != *scope {
            continue;
        }
        let session = binding["session_id"]
            .as_str()
            .ok_or(ContextError::Conflict)?;
        let conversation = value["conversation"]
            .as_str()
            .ok_or(ContextError::Conflict)?;
        sessions.insert((session.to_owned(), conversation.to_owned()));
    }
    for (session, conversation) in sessions {
        let state = crate::context_history::replay(actor, scope, &session, &conversation)
            .await
            .map_err(|e| match e {
                crate::context_history::HistoryError::Context(c) => RetrievalError::Context(c),
                crate::context_history::HistoryError::Ledger(l) => RetrievalError::Ledger(l),
            })?;
        for message in state.history.visible_messages() {
            let span = state.history.source_span(&message.id)?;
            let bytes = state.history.recover(scope, &span)?;
            let Ok(text) = String::from_utf8(bytes) else {
                continue;
            };
            if text.is_empty() || text.len() > MAX_TEXT {
                continue;
            }
            let id = format!(
                "history:{}:{}",
                digest_bytes(session.as_bytes()),
                digest_bytes(message.id.as_bytes())
            );
            let record = SourceRecord {
                scope: scope.clone(),
                kind: SourceKind::Conversation,
                id: id.clone(),
                revision: message.ordinal,
                text: text.clone(),
                content_digest: digest_bytes(text.as_bytes()),
                authority: message.authority,
                provenance: vec![span],
                occurred_at_ns: message.occurred_at_ns,
                recorded_at_ns: message.recorded_at_ns,
                expires_at_ns: None,
                tombstoned: false,
            };
            sources.insert((SourceKind::Conversation, id), record);
        }
    }
    Ok(())
}

pub(super) async fn add_memory(
    actor: &ActorEngine,
    scope: &Scope,
    now_ns: i64,
    sources: &mut BTreeMap<Key, SourceRecord>,
) -> Result<(), RetrievalError> {
    let projection = crate::context_memory::rebuild(actor, scope)
        .await
        .map_err(|e| match e {
            crate::context_memory::MemoryError::Context(c) => RetrievalError::Context(c),
            crate::context_memory::MemoryError::Ledger(l) => RetrievalError::Ledger(l),
        })?;
    for memory in projection
        .records
        .values()
        .filter(|r| r.status == crate::context_memory::RecordStatus::Active)
    {
        let facts: BTreeMap<String, String> = [
            ("owner_id".into(), scope.owner_id.clone()),
            ("project_id".into(), scope.project_id.clone()),
        ]
        .into_iter()
        .collect();
        if projection
            .read_with_facts(scope, &memory.id, now_ns, &facts)
            .map_err(|e| match e {
                crate::context_memory::MemoryError::Context(c) => RetrievalError::Context(c),
                crate::context_memory::MemoryError::Ledger(l) => RetrievalError::Ledger(l),
            })?
            .is_none()
        {
            continue;
        }
        if memory.content.is_empty() || memory.content.len() > MAX_TEXT {
            continue;
        }
        let id = format!("memory:{}", digest_bytes(memory.id.as_bytes()));
        let mut provenance = memory
            .provenance
            .iter()
            .map(|p| SourceSpan {
                source_id: p.source_id.clone(),
                source_digest: p.source_digest.clone(),
                byte_start: p.span_start,
                byte_end: p.span_end,
            })
            .collect::<Vec<_>>();
        if provenance.is_empty() {
            provenance.push(SourceSpan {
                source_id: format!(
                    "memory-source:{}:{}",
                    digest_bytes(memory.id.as_bytes()),
                    memory.revision
                ),
                source_digest: digest_bytes(memory.content.as_bytes()),
                byte_start: 0,
                byte_end: memory.content.len() as u64,
            });
        }
        sources.insert(
            (SourceKind::Memory, id.clone()),
            SourceRecord {
                scope: scope.clone(),
                kind: SourceKind::Memory,
                id,
                revision: memory.revision,
                text: memory.content.clone(),
                content_digest: digest_bytes(memory.content.as_bytes()),
                authority: memory.authority,
                provenance,
                occurred_at_ns: memory.occurred_at_ns,
                recorded_at_ns: memory.recorded_at_ns,
                expires_at_ns: memory.expires_at_ns,
                tombstoned: false,
            },
        );
    }
    Ok(())
}
