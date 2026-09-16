#![allow(clippy::missing_errors_doc)]

use anyhow::{Context, Result, anyhow, ensure};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use clap::Subcommand;
use hm_core::{ActorId, ErrorCode};
use hm_ledger::checkpoint::{
    PublicKey, Signature, checkpoint_path, signing_key_pair_for, verify_archive_manifest_signature,
};
use hm_ledger::keyring::{KeyHierarchy, OsEntropy};
use hm_serve::actor::ActorEngine;
use hm_serve::config::ServerConfig;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub const FORMAT: &str = "hypermind.archive.v1";
pub const ARCHIVE_VERSION: u16 = 1;
pub const OWNED_MEMBERS: [&str; 2] = ["checkpoint.bin", "events.jsonl"];
pub const MAXIMUM_MEMBERS: usize = 8;
pub const MAXIMUM_MEMBER_BYTES: usize = 192 * 1024 * 1024;
pub const MAXIMUM_ARCHIVE_BYTES: usize = 256 * 1024 * 1024;

const CHECKPOINT_MEMBER: &str = "checkpoint.bin";
const EVENTS_MEMBER: &str = "events.jsonl";

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ArchiveMember {
    pub name: String,
    pub bytes: u64,
    pub digest: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ArchiveManifest {
    pub format: String,
    pub archive_version: u16,
    pub actor: u16,
    pub events: u64,
    pub created_ns: i64,
    pub members: Vec<ArchiveMember>,
    pub public_key: String,
    pub signature: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct MemberLine {
    member: String,
    content_base64: String,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    Pack {
        #[arg(long)]
        config: PathBuf,
        #[arg(long)]
        output: PathBuf,
    },
    Unpack {
        #[arg(long)]
        input: PathBuf,
        #[arg(long)]
        output: PathBuf,
    },
    Verify {
        #[arg(long)]
        input: PathBuf,
        #[arg(long)]
        public_key: Option<String>,
    },
}

pub async fn execute(command: Command) -> Result<Value> {
    match command {
        Command::Pack { config, output } => pack(&config, &output).await,
        Command::Unpack { input, output } => unpack(&input, &output),
        Command::Verify { input, public_key } => {
            let (manifest, members) = verify_archive(&input, public_key.as_deref())?;
            Ok(json!({
                "ok": true,
                "verified": true,
                "format": manifest.format,
                "archive_version": manifest.archive_version,
                "actor": manifest.actor,
                "events": manifest.events,
                "created_ns": manifest.created_ns,
                "members": members.iter().map(|(name, content)| json!({"name":name,"bytes":content.len()})).collect::<Vec<_>>(),
                "public_key": manifest.public_key,
                "pinned_public_key": public_key.is_some(),
                "display": format!(
                    "ARCHIVE VERIFIED actor={} events={} members={}",
                    manifest.actor,
                    manifest.events,
                    members.len()
                ),
            }))
        }
    }
}

#[must_use]
pub fn manifest_digest(manifest: &ArchiveManifest) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new();
    hasher.update(FORMAT.as_bytes());
    hasher.update(b"\n");
    hasher.update(manifest.archive_version.to_string().as_bytes());
    hasher.update(b"\n");
    hasher.update(manifest.actor.to_string().as_bytes());
    hasher.update(b"\n");
    let mut members = manifest.members.clone();
    members.sort_by(|left, right| left.name.cmp(&right.name));
    for member in &members {
        hasher.update(member.name.as_bytes());
        hasher.update(b"\n");
        hasher.update(member.bytes.to_string().as_bytes());
        hasher.update(b"\n");
        hasher.update(member.digest.as_bytes());
        hasher.update(b"\n");
    }
    *hasher.finalize().as_bytes()
}

pub async fn pack(config: &Path, output: &Path) -> Result<Value> {
    let _lock = crate::actors::operation_lock(config)?;
    let server = hm_serve::config::load(config)?;
    crate::actors::require_offline(&server)?;
    let actor_id = crate::first_actor(&server)?.actor;
    let actor = crate::actors::open(&server, actor_id).await?;
    let result = collect_members(&server, &actor, actor_id).await;
    actor.shutdown().await?;
    let (members, events) = result?;
    let archive = seal(&server, actor_id, events, &members)?;
    write_archive(output, &archive.0)?;
    Ok(json!({
        "ok": true,
        "format": FORMAT,
        "archive_version": ARCHIVE_VERSION,
        "actor": actor_id,
        "events": events,
        "members": archive.1.members,
        "public_key": archive.1.public_key,
        "bytes": archive.0.len(),
    }))
}

#[allow(clippy::type_complexity)]
pub fn verify_archive(
    input: &Path,
    pinned_public_key: Option<&str>,
) -> Result<(ArchiveManifest, Vec<(String, Vec<u8>)>)> {
    let metadata = std::fs::metadata(input)?;
    ensure!(
        metadata.len() <= u64::try_from(MAXIMUM_ARCHIVE_BYTES)?,
        "archive exceeds the maximum archive size"
    );
    let bytes = std::fs::read(input)?;
    ensure!(
        bytes.len() <= MAXIMUM_ARCHIVE_BYTES,
        "archive exceeds the maximum archive size"
    );
    let text = std::str::from_utf8(&bytes)?;
    let mut lines = text.lines();
    let manifest: ArchiveManifest = serde_json::from_str(lines.next().context("empty archive")?)?;
    ensure!(manifest.format == FORMAT, "unrecognized archive format");
    ensure!(
        manifest.archive_version == ARCHIVE_VERSION,
        "unsupported archive version"
    );
    ensure!(
        !manifest.members.is_empty() && manifest.members.len() <= MAXIMUM_MEMBERS,
        "archive member count is outside the permitted range"
    );
    for (index, member) in manifest.members.iter().enumerate() {
        ensure!(
            OWNED_MEMBERS.contains(&member.name.as_str()),
            "archive member name is outside the owned-member allowlist"
        );
        ensure!(
            !manifest.members[..index]
                .iter()
                .any(|earlier| earlier.name == member.name),
            "archive names the same member twice"
        );
        ensure!(
            member.bytes <= u64::try_from(MAXIMUM_MEMBER_BYTES)?,
            "archive member exceeds the maximum member size"
        );
        ensure!(
            member.digest.len() == 64
                && member
                    .digest
                    .bytes()
                    .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase()),
            "archive member digest is not lowercase blake3 hex"
        );
    }
    let public_key: PublicKey = decode_hex(&manifest.public_key)?;
    let signature: Signature = decode_hex(&manifest.signature)?;
    if let Some(pinned) = pinned_public_key {
        let pinned: PublicKey = decode_hex(pinned)?;
        ensure!(
            pinned == public_key,
            "archive public key does not match the pinned public key"
        );
    }
    verify_archive_manifest_signature(&manifest_digest(&manifest), &signature, &public_key)?;
    let mut members = Vec::with_capacity(manifest.members.len());
    for line in lines {
        let record: MemberLine = serde_json::from_str(line)?;
        let declared = manifest
            .members
            .get(members.len())
            .context("archive carries more members than the manifest declares")?;
        ensure!(
            record.member == declared.name,
            "archive member order differs from the manifest"
        );
        ensure!(
            record.content_base64.len() / 4 * 3 <= MAXIMUM_MEMBER_BYTES,
            "archive member exceeds the maximum member size"
        );
        let content = STANDARD.decode(record.content_base64.as_bytes())?;
        ensure!(
            u64::try_from(content.len())? == declared.bytes,
            "archive member length differs from the manifest"
        );
        ensure!(
            hex(blake3::hash(&content).as_bytes()) == declared.digest,
            "archive member digest differs from the manifest"
        );
        members.push((declared.name.clone(), content));
    }
    ensure!(
        members.len() == manifest.members.len(),
        "archive is missing a member the manifest declares"
    );
    Ok((manifest, members))
}

pub fn validate_member_name(name: &str) -> Result<&'static str> {
    ensure!(!name.is_empty(), "archive member name is empty");
    ensure!(
        !name.contains('/')
            && !name.contains('\\')
            && !name.contains(':')
            && !name.contains('\0')
            && !name.contains(".."),
        "archive member name carries a path component"
    );
    ensure!(
        name.bytes().all(|byte| byte.is_ascii_lowercase()
            || byte.is_ascii_digit()
            || matches!(byte, b'.' | b'_' | b'-')),
        "archive member name carries a character outside the permitted set"
    );
    OWNED_MEMBERS
        .into_iter()
        .find(|owned| *owned == name)
        .context("archive member name is outside the owned-member allowlist")
}

pub fn unpack(input: &Path, output: &Path) -> Result<Value> {
    let (manifest, members) = verify_archive(input, None)?;
    ensure!(
        std::fs::metadata(output)?.is_dir(),
        "archive output must be an existing directory"
    );
    let mut targets = Vec::with_capacity(members.len());
    for (index, (name, content)) in members.iter().enumerate() {
        let owned = validate_member_name(name)?;
        ensure!(
            !members[..index]
                .iter()
                .any(|(earlier, _)| earlier.as_str() == owned),
            "archive names the same member twice"
        );
        let path = output.join(owned);
        ensure!(
            path.parent() == Some(output),
            "archive member does not land directly in the output directory"
        );
        targets.push((owned, path, content));
    }
    let mut written = Vec::with_capacity(targets.len());
    for (name, path, content) in targets {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&path)?;
        file.write_all(content)?;
        file.sync_all()?;
        written.push(json!({"name": name, "bytes": content.len(), "path": path}));
    }
    File::open(output)?.sync_all()?;
    let count = written.len();
    Ok(json!({
        "ok": true,
        "verified": true,
        "format": manifest.format,
        "actor": manifest.actor,
        "members": written,
        "display": format!(
            "ARCHIVE UNPACKED actor={} members={count}",
            manifest.actor
        ),
    }))
}

async fn collect_members(
    server: &ServerConfig,
    actor: &ActorEngine,
    actor_id: u16,
) -> Result<(Vec<(String, Vec<u8>)>, u64)> {
    let (events, count) = crate::transfer::export_stream(actor).await?;
    let mut members = Vec::new();
    let checkpoint_lsn = actor.verification_status().await?.last_checkpoint_lsn;
    if checkpoint_lsn.get() != 0 {
        let path = checkpoint_path(
            &server.actor_directory(actor_id).join("mmr/checkpoints"),
            checkpoint_lsn,
        );
        if path.exists() {
            members.push((CHECKPOINT_MEMBER.to_string(), std::fs::read(&path)?));
        }
    }
    members.push((EVENTS_MEMBER.to_string(), events));
    members.sort_by(|left, right| left.0.cmp(&right.0));
    Ok((members, count))
}

fn seal(
    server: &ServerConfig,
    actor_id: u16,
    events: u64,
    members: &[(String, Vec<u8>)],
) -> Result<(Vec<u8>, ArchiveManifest)> {
    ensure!(
        members.len() <= MAXIMUM_MEMBERS,
        "archive member count exceeds the permitted maximum"
    );
    let mut records = Vec::with_capacity(members.len());
    for (name, content) in members {
        ensure!(
            OWNED_MEMBERS.contains(&name.as_str()),
            "refusing to pack a member outside the owned-member allowlist"
        );
        ensure!(
            content.len() <= MAXIMUM_MEMBER_BYTES,
            "a member of this actor exceeds the maximum member size; this version cannot archive it"
        );
        records.push(ArchiveMember {
            name: name.clone(),
            bytes: u64::try_from(content.len())?,
            digest: hex(blake3::hash(content).as_bytes()),
        });
    }
    let keys = KeyHierarchy::open_or_create(
        server.actor_directory(actor_id),
        ActorId::new(actor_id),
        server.user,
        &server.kek,
        &mut OsEntropy,
        false,
    )?;
    let pair = signing_key_pair_for(&keys);
    let mut manifest = ArchiveManifest {
        format: FORMAT.to_string(),
        archive_version: ARCHIVE_VERSION,
        actor: actor_id,
        events,
        created_ns: created_ns()?,
        members: records,
        public_key: hex(&pair.public_key),
        signature: String::new(),
    };
    manifest.signature = hex(&pair.sign_archive_manifest(&manifest_digest(&manifest)));
    let mut archive = Vec::new();
    writeln!(archive, "{}", serde_json::to_string(&manifest)?)?;
    for (name, content) in members {
        writeln!(
            archive,
            "{}",
            serde_json::to_string(&MemberLine {
                member: name.clone(),
                content_base64: STANDARD.encode(content),
            })?
        )?;
        ensure!(
            archive.len() <= MAXIMUM_ARCHIVE_BYTES,
            "this actor would produce an archive larger than the maximum archive size; nothing was written"
        );
    }
    Ok((archive, manifest))
}

fn write_archive(output: &Path, archive: &[u8]) -> Result<()> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(output)?;
    file.write_all(archive)?;
    file.sync_all()?;
    File::open(output.parent().context("output directory missing")?)?.sync_all()?;
    Ok(())
}

fn created_ns() -> Result<i64> {
    Ok(i64::try_from(
        SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos(),
    )?)
}

fn decode_hex<const N: usize>(value: &str) -> Result<[u8; N]> {
    if value.len() != N * 2 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(anyhow!(ErrorCode::InvalidArgument));
    }
    let mut decoded = [0_u8; N];
    for (index, byte) in decoded.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&value[index * 2..index * 2 + 2], 16)
            .map_err(|_| anyhow!(ErrorCode::InvalidArgument))?;
    }
    Ok(decoded)
}

fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    bytes.iter().fold(
        String::with_capacity(bytes.len() * 2),
        |mut output, byte| {
            let _ = write!(output, "{byte:02x}");
            output
        },
    )
}
