use crate::{
    ContextError, Cursor, Scope,
    types::{digest_bytes, validate_id},
};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
};

const VERSION: u32 = 1;
const MAX_RECORDS: usize = 1024;
const MAX_BYTES: u64 = 4 * 1024 * 1024;
#[derive(Debug, thiserror::Error)]
pub enum JournalError {
    #[error("journal state: {0}")]
    Context(#[from] ContextError),
    #[error("journal storage: {0}")]
    Io(#[from] std::io::Error),
    #[error("journal encoding: {0}")]
    Json(#[from] serde_json::Error),
    #[error("pending mutation must be reconciled")]
    Busy,
    #[error("journal storage outcome requires reopen")]
    Poisoned,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JournalBinding {
    pub scope: Scope,
    pub device_id: String,
    pub session_id: String,
}
impl JournalBinding {
    fn validate(&self) -> Result<(), JournalError> {
        self.scope.validate()?;
        validate_id(&self.device_id)?;
        validate_id(&self.session_id)?;
        Ok(())
    }
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MutationIdentity {
    pub binding: JournalBinding,
    pub operation_id: String,
    pub operation_kind: String,
    pub payload_digest: String,
}
impl MutationIdentity {
    fn validate(&self) -> Result<(), JournalError> {
        self.binding.validate()?;
        validate_id(&self.operation_id)?;
        validate_id(&self.operation_kind)?;
        digest(&self.payload_digest)
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MutationState {
    Pending,
    Unknown,
    Terminal,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReceiptOutcome {
    Succeeded,
    Failed,
    NotApplied,
    Unknown,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NeutralReceipt {
    pub identity: MutationIdentity,
    pub intent_version: u64,
    pub cursor: Cursor,
    pub receipt_digest: String,
    pub outcome: ReceiptOutcome,
    pub observed_session_id: String,
}
impl NeutralReceipt {
    fn validate(&self) -> Result<(), JournalError> {
        self.identity.validate()?;
        self.cursor.validate()?;
        if self.intent_version == 0 || self.intent_version > ((1u64 << 53) - 1) {
            return Err(ContextError::Invalid("invalid intent version".into()).into());
        }
        digest(&self.receipt_digest)?;
        validate_id(&self.observed_session_id)?;
        Ok(())
    }
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct WakeIdentifier(String);
impl WakeIdentifier {
    pub fn as_str(&self) -> &str {
        &self.0
    }
    pub fn parse(value: &str) -> Result<Self, JournalError> {
        digest(value)?;
        Ok(Self(value.into()))
    }
    fn fresh() -> Result<Self, JournalError> {
        let mut entropy = [0u8; 32];
        File::open("/dev/urandom")?.read_exact(&mut entropy)?;
        Ok(Self(digest_bytes(&entropy)))
    }
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MutationRecord {
    pub identity: MutationIdentity,
    pub state: MutationState,
    pub dispatched_session_id: Option<String>,
    pub receipt: Option<NeutralReceipt>,
    pub wake_id: WakeIdentifier,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Snapshot {
    version: u32,
    binding: JournalBinding,
    revision: u64,
    cursor: Option<Cursor>,
    records: Vec<MutationRecord>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct DiskSnapshot {
    snapshot: Snapshot,
    digest: String,
}
pub struct MobileJournal {
    path: PathBuf,
    snapshot: Snapshot,
    _lock: File,
    resume_required: bool,
    poisoned: bool,
}
fn digest(value: &str) -> Result<(), JournalError> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(ContextError::Invalid("invalid digest".into()).into());
    }
    Ok(())
}
fn conflict() -> JournalError {
    ContextError::Conflict.into()
}
fn ordinary(path: &Path) -> Result<(), JournalError> {
    if let Ok(metadata) = fs::symlink_metadata(path) {
        if !metadata.file_type().is_file() {
            return Err(ContextError::Invalid("ordinary journal file required".into()).into());
        }
    }
    Ok(())
}
impl MobileJournal {
    pub fn open(path: impl AsRef<Path>, binding: JournalBinding) -> Result<Self, JournalError> {
        binding.validate()?;
        let path = path.as_ref().to_path_buf();
        let parent = path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        fs::create_dir_all(parent)?;
        ordinary(&path)?;
        let mut lock_path = path.as_os_str().to_os_string();
        lock_path.push(".lock");
        let lock_path = PathBuf::from(lock_path);
        ordinary(&lock_path)?;
        let lock = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&lock_path)?;
        lock.try_lock().map_err(|_| conflict())?;
        let existed = path.exists();
        let snapshot = if existed {
            let mut bytes = Vec::new();
            File::open(&path)?
                .take(MAX_BYTES + 1)
                .read_to_end(&mut bytes)?;
            if bytes.len() as u64 > MAX_BYTES {
                return Err(ContextError::Capacity.into());
            }
            let disk: DiskSnapshot = serde_json::from_slice(&bytes)?;
            if disk.digest != digest_bytes(&serde_json::to_vec(&disk.snapshot)?) {
                return Err(conflict());
            }
            disk.snapshot
        } else {
            Snapshot {
                version: VERSION,
                binding: binding.clone(),
                revision: 0,
                cursor: None,
                records: vec![],
            }
        };
        Self::validate_snapshot(&snapshot)?;
        if snapshot.binding != binding {
            return Err(ContextError::ScopeMismatch.into());
        }
        let resume_required = snapshot
            .records
            .iter()
            .any(|r| r.state == MutationState::Unknown);
        let mut journal = Self {
            path,
            snapshot,
            _lock: lock,
            resume_required,
            poisoned: false,
        };
        if !existed {
            journal.persist(journal.snapshot.clone())?;
        }
        Ok(journal)
    }
    fn validate_snapshot(snapshot: &Snapshot) -> Result<(), JournalError> {
        snapshot.binding.validate()?;
        if snapshot.version != VERSION || snapshot.records.len() > MAX_RECORDS {
            return Err(ContextError::Capacity.into());
        }
        if let Some(cursor) = snapshot.cursor {
            cursor.validate()?;
        }
        let mut ids = std::collections::HashSet::new();
        let mut blocked = 0;
        let mut seen_receipt_cursor = None;
        for record in &snapshot.records {
            record.identity.validate()?;
            WakeIdentifier::parse(record.wake_id.as_str())?;
            if record.identity.binding != snapshot.binding
                || !ids.insert(record.identity.operation_id.clone())
            {
                return Err(conflict());
            }
            if let Some(session) = &record.dispatched_session_id {
                validate_id(session)?;
            }
            match record.state {
                MutationState::Pending => {
                    if record.dispatched_session_id.is_some() || record.receipt.is_some() {
                        return Err(conflict());
                    }
                    blocked += 1;
                }
                MutationState::Unknown => {
                    if record.dispatched_session_id.is_none() {
                        return Err(conflict());
                    }
                    blocked += 1;
                }
                MutationState::Terminal => {
                    if record.dispatched_session_id.is_none()
                        || record
                            .receipt
                            .as_ref()
                            .is_none_or(|r| r.outcome == ReceiptOutcome::Unknown)
                    {
                        return Err(conflict());
                    }
                }
            }
            if let Some(receipt) = &record.receipt {
                receipt.validate()?;
                if receipt.identity != record.identity
                    || seen_receipt_cursor.is_some_and(|cursor| receipt.cursor < cursor)
                {
                    return Err(conflict());
                }
                if record.state == MutationState::Unknown
                    && receipt.outcome != ReceiptOutcome::Unknown
                {
                    return Err(conflict());
                }
                seen_receipt_cursor = Some(receipt.cursor);
            }
        }
        if blocked > 1 || snapshot.cursor != seen_receipt_cursor {
            return Err(conflict());
        }
        Ok(())
    }
    fn persist(&mut self, snapshot: Snapshot) -> Result<(), JournalError> {
        Self::validate_snapshot(&snapshot)?;
        let checksum = digest_bytes(&serde_json::to_vec(&snapshot)?);
        let bytes = serde_json::to_vec(&DiskSnapshot {
            snapshot: snapshot.clone(),
            digest: checksum,
        })?;
        if bytes.len() as u64 > MAX_BYTES {
            return Err(ContextError::Capacity.into());
        }
        let parent = self
            .path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        let name = self
            .path
            .file_name()
            .ok_or_else(|| ContextError::Invalid("journal path".into()))?
            .to_string_lossy();
        let temp = parent.join(format!(
            ".{name}.{}.pending",
            WakeIdentifier::fresh()?.as_str()
        ));
        let write = || -> Result<(), JournalError> {
            let mut options = OpenOptions::new();
            options.write(true).create_new(true);
            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt;
                options.mode(0o600);
            }
            let mut file = options.open(&temp)?;
            file.write_all(&bytes)?;
            file.sync_all()?;
            fs::rename(&temp, &self.path)?;
            File::open(parent)?.sync_all()?;
            Ok(())
        };
        if let Err(error) = write() {
            let _ = fs::remove_file(&temp);
            self.poisoned = true;
            return Err(error);
        }
        self.snapshot = snapshot;
        Ok(())
    }
    fn ensure_active(&self) -> Result<(), JournalError> {
        if self.poisoned {
            Err(JournalError::Poisoned)
        } else {
            Ok(())
        }
    }
    pub fn enqueue_mutation(
        &mut self,
        operation_id: &str,
        operation_kind: &str,
        payload_digest: &str,
    ) -> Result<MutationRecord, JournalError> {
        self.ensure_active()?;
        let identity = MutationIdentity {
            binding: self.snapshot.binding.clone(),
            operation_id: operation_id.into(),
            operation_kind: operation_kind.into(),
            payload_digest: payload_digest.into(),
        };
        identity.validate()?;
        if self
            .snapshot
            .records
            .iter()
            .any(|r| r.identity.operation_id == operation_id)
        {
            return Err(conflict());
        }
        if self.pending().is_some() {
            return Err(JournalError::Busy);
        }
        if self.snapshot.records.len() == MAX_RECORDS {
            return Err(ContextError::Capacity.into());
        }
        let record = MutationRecord {
            identity,
            state: MutationState::Pending,
            dispatched_session_id: None,
            receipt: None,
            wake_id: WakeIdentifier::fresh()?,
        };
        let mut next = self.snapshot.clone();
        next.revision = next.revision.checked_add(1).ok_or(ContextError::Capacity)?;
        next.records.push(record.clone());
        self.persist(next)?;
        Ok(record)
    }
    pub fn mark_dispatched(
        &mut self,
        operation_id: &str,
        session_id: &str,
    ) -> Result<MutationRecord, JournalError> {
        self.ensure_active()?;
        validate_id(session_id)?;
        let mut next = self.snapshot.clone();
        let record = next
            .records
            .iter_mut()
            .find(|r| r.identity.operation_id == operation_id)
            .ok_or(ContextError::Conflict)?;
        if record.state != MutationState::Pending {
            return Err(conflict());
        }
        record.state = MutationState::Unknown;
        record.dispatched_session_id = Some(session_id.into());
        let output = record.clone();
        next.revision = next.revision.checked_add(1).ok_or(ContextError::Capacity)?;
        self.persist(next)?;
        self.resume_required = false;
        Ok(output)
    }
    pub fn reconcile_before_next_write(
        &mut self,
        receipt: &NeutralReceipt,
    ) -> Result<MutationRecord, JournalError> {
        self.ensure_active()?;
        receipt.validate()?;
        if receipt.identity.binding != self.snapshot.binding {
            return Err(ContextError::ScopeMismatch.into());
        }
        if self
            .snapshot
            .cursor
            .is_some_and(|cursor| receipt.cursor < cursor)
        {
            return Err(ContextError::Stale.into());
        }
        let mut next = self.snapshot.clone();
        let record = next
            .records
            .iter_mut()
            .find(|r| r.identity.operation_id == receipt.identity.operation_id)
            .ok_or(ContextError::Conflict)?;
        if record.identity != receipt.identity {
            return Err(conflict());
        }
        if record.state == MutationState::Terminal {
            return if record.receipt.as_ref() == Some(receipt) {
                Ok(record.clone())
            } else {
                Err(conflict())
            };
        }
        if record.state != MutationState::Unknown {
            return Err(conflict());
        }
        if self.resume_required
            && record.dispatched_session_id.as_deref() == Some(&receipt.observed_session_id)
        {
            return Err(ContextError::Stale.into());
        }
        if let Some(previous) = &record.receipt {
            if receipt.intent_version < previous.intent_version
                || (receipt.intent_version == previous.intent_version
                    && receipt.receipt_digest != previous.receipt_digest)
            {
                return Err(conflict());
            }
        }
        record.state = if receipt.outcome == ReceiptOutcome::Unknown {
            MutationState::Unknown
        } else {
            MutationState::Terminal
        };
        record.receipt = Some(receipt.clone());
        let output = record.clone();
        next.cursor = Some(receipt.cursor);
        next.revision = next.revision.checked_add(1).ok_or(ContextError::Capacity)?;
        self.persist(next)?;
        if receipt.outcome == ReceiptOutcome::Unknown {
            self.resume_required = true;
        }
        Ok(output)
    }
    pub fn cursor_receipt(&self) -> Option<Cursor> {
        self.snapshot.cursor
    }
    pub fn pending(&self) -> Option<&MutationRecord> {
        self.snapshot
            .records
            .iter()
            .find(|r| r.state != MutationState::Terminal)
    }
    pub fn records(&self) -> &[MutationRecord] {
        &self.snapshot.records
    }
}
