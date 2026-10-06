#![allow(clippy::missing_errors_doc)]
use crate::{
    checkpoint::{PublicKey, SigningKeyPair, verify_archive_manifest_signature},
    frame::{Frame, encode},
    keyring::{io_error, sync_directory},
    mmr::{Hash, hash_bytes, hash_frame_sealed},
    segment::{AppendRequest, SegmentLog, SegmentLogOptions},
};
use hm_core::{ActorId, Error, ErrorCode};
use std::{
    collections::BTreeMap,
    fs::{self, OpenOptions},
    io::Write,
    os::unix::fs::OpenOptionsExt,
    path::{Path, PathBuf},
};
const MAGIC: &[u8; 8] = b"HMPURG01";
#[derive(Clone, Debug)]
pub struct CertifiedRedaction {
    pub lsn: u64,
    pub original: Hash,
    pub replacement: Hash,
}
#[derive(Clone, Debug)]
pub struct Certificate {
    pub actor: u16,
    pub leaf_count: u64,
    pub root: Hash,
    pub plan: Hash,
    pub entries: Vec<CertifiedRedaction>,
}
#[derive(Clone, Debug)]
pub struct StorageReceipt {
    pub plan: Hash,
    pub original_root: Hash,
    pub redacted_frames: usize,
    pub removed_generations: usize,
}
fn invalid() -> Error {
    Error::new(ErrorCode::CheckpointMismatch)
}
fn write_sync(path: &Path, bytes: &[u8]) -> Result<(), Error> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)
        .map_err(|e| io_error(ErrorCode::OpenFailed, &e))?;
    file.write_all(bytes)
        .map_err(|e| io_error(ErrorCode::WriteFailed, &e))?;
    file.sync_all()
        .map_err(|e| io_error(ErrorCode::SyncFailed, &e))
}
fn root(actor: &Path) -> PathBuf {
    actor.join("retention")
}
pub fn active_directory(actor: &Path) -> Result<PathBuf, Error> {
    let path = root(actor).join("CURRENT");
    match fs::read_to_string(path) {
        Ok(id) => {
            if id.len() != 64 || !id.bytes().all(|b| b.is_ascii_hexdigit()) {
                return Err(invalid());
            }
            Ok(root(actor).join("generations").join(id))
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(actor.to_owned()),
        Err(e) => Err(io_error(ErrorCode::ReadFailed, &e)),
    }
}
fn body(c: &Certificate) -> Vec<u8> {
    let mut b = Vec::new();
    b.extend_from_slice(MAGIC);
    b.extend_from_slice(&c.actor.to_le_bytes());
    b.extend_from_slice(&c.leaf_count.to_le_bytes());
    b.extend_from_slice(&c.root);
    b.extend_from_slice(&c.plan);
    b.extend_from_slice(&(c.entries.len() as u64).to_le_bytes());
    for e in &c.entries {
        b.extend_from_slice(&e.lsn.to_le_bytes());
        b.extend_from_slice(&e.original);
        b.extend_from_slice(&e.replacement);
    }
    b
}
pub fn load(actor: &Path, key: &PublicKey) -> Result<Option<Certificate>, Error> {
    let dir = active_directory(actor)?;
    if dir == actor {
        return Ok(None);
    }
    let b = fs::read(dir.join("REDACTIONS")).map_err(|e| io_error(ErrorCode::ReadFailed, &e))?;
    if b.len() < 154 || &b[..8] != MAGIC {
        return Err(invalid());
    }
    let count = u64::from_le_bytes(b[82..90].try_into().map_err(|_| invalid())?);
    let count = usize::try_from(count).map_err(|_| invalid())?;
    let end = 90usize
        .checked_add(count.checked_mul(72).ok_or_else(invalid)?)
        .ok_or_else(invalid)?;
    if b.len() != end + 64 {
        return Err(invalid());
    }
    let signature = b[end..].try_into().map_err(|_| invalid())?;
    verify_archive_manifest_signature(&hash_bytes(&b[..end]), &signature, key)?;
    let mut entries = Vec::new();
    let mut prev = 0;
    for chunk in b[90..end].chunks_exact(72) {
        let lsn = u64::from_le_bytes(chunk[..8].try_into().map_err(|_| invalid())?);
        if lsn <= prev || lsn > u64::from_le_bytes(b[10..18].try_into().map_err(|_| invalid())?) {
            return Err(invalid());
        }
        prev = lsn;
        entries.push(CertifiedRedaction {
            lsn,
            original: chunk[8..40].try_into().map_err(|_| invalid())?,
            replacement: chunk[40..72].try_into().map_err(|_| invalid())?,
        });
    }
    Ok(Some(Certificate {
        actor: u16::from_le_bytes(b[8..10].try_into().map_err(|_| invalid())?),
        leaf_count: u64::from_le_bytes(b[10..18].try_into().map_err(|_| invalid())?),
        root: b[18..50].try_into().map_err(|_| invalid())?,
        plan: b[50..82].try_into().map_err(|_| invalid())?,
        entries,
    }))
}
pub fn verify_frame(c: &Certificate, frame: &Frame, committed: Hash) -> Result<(), Error> {
    let hash = hash_frame_sealed(&frame.header, &frame.sealed_payload);
    if let Some(e) = c.entries.iter().find(|e| e.lsn == frame.header.lsn.get()) {
        if e.original != committed || e.replacement != hash {
            return Err(invalid().at_lsn(frame.header.lsn));
        }
    } else if committed != hash {
        return Err(invalid().at_lsn(frame.header.lsn));
    }
    Ok(())
}
pub fn switch_generation(
    actor: &Path,
    actor_id: ActorId,
    frames: &[Frame],
    replacements: &BTreeMap<u64, Vec<u8>>,
    root_hash: Hash,
    plan: Hash,
    keys: &SigningKeyPair,
) -> Result<(), Error> {
    if replacements.is_empty() {
        return Err(Error::new(ErrorCode::InvalidArgument));
    }
    let current = load(actor, &keys.public_key)?;
    let mut entries: BTreeMap<u64, CertifiedRedaction> = current
        .into_iter()
        .flat_map(|c| c.entries)
        .map(|e| (e.lsn, e))
        .collect();
    let mut requests = Vec::new();
    for frame in frames {
        let sealed = if let Some(value) = replacements.get(&frame.header.lsn.get()) {
            let original = entries.get(&frame.header.lsn.get()).map_or_else(
                || hash_frame_sealed(&frame.header, &frame.sealed_payload),
                |e| e.original,
            );
            let next = Frame {
                header: frame.header,
                sealed_payload: value.clone(),
            };
            entries.insert(
                frame.header.lsn.get(),
                CertifiedRedaction {
                    lsn: frame.header.lsn.get(),
                    original,
                    replacement: hash_frame_sealed(&next.header, &next.sealed_payload),
                },
            );
            value.clone()
        } else {
            frame.sealed_payload.clone()
        };
        requests.push(AppendRequest {
            kind: frame.header.kind,
            wall_timestamp_ns: frame.header.wall_timestamp_ns,
            conversation: frame.header.conversation,
            sealed_payload: sealed,
        });
    }
    let id = plan.iter().map(|b| format!("{b:02x}")).collect::<String>();
    let base = root(actor);
    let generations = base.join("generations");
    fs::create_dir_all(&generations).map_err(|e| io_error(ErrorCode::OpenFailed, &e))?;
    let new_dir = generations.join(&id);
    if new_dir.exists() {
        fs::remove_dir_all(&new_dir).map_err(|e| io_error(ErrorCode::WriteFailed, &e))?
    }
    fs::create_dir(&new_dir).map_err(|e| io_error(ErrorCode::OpenFailed, &e))?;
    let mut new_log = SegmentLog::open(&new_dir, actor_id, SegmentLogOptions::default())?;
    new_log.append_batch(&requests)?;
    let expected: Vec<Vec<u8>> = frames
        .iter()
        .filter(|f| !replacements.contains_key(&f.header.lsn.get()))
        .map(encode)
        .collect::<Result<_, _>>()?;
    let actual: Vec<Vec<u8>> = new_log
        .read_all()?
        .iter()
        .filter(|f| !replacements.contains_key(&f.header.lsn.get()))
        .map(encode)
        .collect::<Result<_, _>>()?;
    if expected != actual {
        return Err(invalid());
    }
    let c = Certificate {
        actor: actor_id.get(),
        leaf_count: frames.len() as u64,
        root: root_hash,
        plan,
        entries: entries.into_values().collect(),
    };
    let mut encoded = body(&c);
    encoded.extend_from_slice(&keys.sign_archive_manifest(&hash_bytes(&encoded)));
    write_sync(&new_dir.join("REDACTIONS"), &encoded)?;
    sync_directory(&new_dir)?;
    sync_directory(&generations)?;
    let pending = base.join("CURRENT.pending");
    if pending.exists() {
        fs::remove_file(&pending).map_err(|e| io_error(ErrorCode::WriteFailed, &e))?
    }
    write_sync(&pending, id.as_bytes())?;
    fs::rename(&pending, base.join("CURRENT")).map_err(|e| io_error(ErrorCode::WriteFailed, &e))?;
    sync_directory(&base)
}
pub fn finish_cleanup(actor: &Path) -> Result<usize, Error> {
    let active = active_directory(actor)?;
    if active == actor {
        return Ok(0);
    }
    let mut removed = 0;
    for name in ["log", "projections", "vectors"] {
        let path = actor.join(name);
        if path.exists() {
            fs::remove_dir_all(path).map_err(|e| io_error(ErrorCode::WriteFailed, &e))?;
            removed += 1;
        }
    }
    let generations = root(actor).join("generations");
    for entry in fs::read_dir(&generations).map_err(|e| io_error(ErrorCode::ReadFailed, &e))? {
        let entry = entry.map_err(|e| io_error(ErrorCode::ReadFailed, &e))?;
        if entry.path() != active {
            fs::remove_dir_all(entry.path()).map_err(|e| io_error(ErrorCode::WriteFailed, &e))?;
            removed += 1;
        }
    }
    sync_directory(&generations)?;
    sync_directory(actor)?;
    Ok(removed)
}
