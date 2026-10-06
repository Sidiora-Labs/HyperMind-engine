use ed25519_dalek::{Signature, VerifyingKey};
use hm_context::{digest_bytes, validate_id, Scope};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, io::{Read, Write}, path::{Component, Path}, fs::{self, OpenOptions}};

#[derive(Debug, thiserror::Error)]
pub enum ArtifactError {
    #[error("invalid artifact: {0}")] Invalid(String),
    #[error("artifact signature not trusted")] Signature,
    #[error("artifact digest mismatch")] Digest,
    #[error("artifact capacity exceeded")] Capacity,
    #[error("artifact I/O: {0}")] Io(#[from] std::io::Error),
    #[error("artifact serialization: {0}")] Json(#[from] serde_json::Error),
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArtifactFile { pub digest: String, pub bytes: u64, pub executable: bool }
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArtifactManifest {
    pub format_version: u32,
    pub scope: Scope,
    pub module_id: String,
    pub version: String,
    pub target_os: String,
    pub target_arch: String,
    pub protocol_version: u32,
    pub archive_digest: String,
    pub archive_bytes: u64,
    pub files: BTreeMap<String, ArtifactFile>,
    pub capabilities: BTreeMap<String,u32>,
    pub command: String,
    pub args: Vec<String>,
    pub env: BTreeMap<String,String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SignedArtifact { pub manifest: ArtifactManifest, pub signer: [u8;32], pub signature: Vec<u8> }
#[derive(Clone, Copy, Debug)]
pub struct ArtifactLimits { pub archive_bytes: u64, pub extracted_bytes: u64, pub file_count: usize }
impl Default for ArtifactLimits { fn default()->Self { Self{archive_bytes:64*1024*1024,extracted_bytes:64*1024*1024,file_count:4096} } }
#[derive(Clone, Debug)]
pub struct VerifiedArtifact { manifest: ArtifactManifest, manifest_digest: String }
impl VerifiedArtifact {
    pub fn manifest(&self)->&ArtifactManifest { &self.manifest }
    pub fn digest(&self)->&str { &self.manifest_digest }
}
pub fn safe_relative(path:&str)->Result<(),ArtifactError> {
    if path.is_empty() || path.len()>512 || path.contains(['\\','\0']) || Path::new(path).components().any(|c| !matches!(c,Component::Normal(_))) {
        return Err(ArtifactError::Invalid("unsafe relative path".into()));
    }
    Ok(())
}
pub fn manifest_bytes(manifest:&ArtifactManifest)->Result<Vec<u8>,ArtifactError> { Ok(serde_json::to_vec(manifest)?) }
pub fn verify_artifact(signed:&SignedArtifact, trusted:&VerifyingKey, scope:&Scope, archive:&[u8], limits:ArtifactLimits)->Result<VerifiedArtifact,ArtifactError> {
    let m=&signed.manifest;
    if &m.scope!=scope || m.scope.validate().is_err() || m.format_version!=1 || m.protocol_version!=1 || m.target_os!=std::env::consts::OS || m.target_arch!=std::env::consts::ARCH || validate_id(&m.module_id).is_err() || validate_id(&m.version).is_err() || m.files.is_empty() || m.files.len()>limits.file_count || m.capabilities.len()>128 || m.args.len()>128 || m.env.len()>128 { return Err(ArtifactError::Invalid("manifest compatibility or bounds".into())); }
    if archive.len() as u64>limits.archive_bytes || m.archive_bytes!=archive.len() as u64 { return Err(ArtifactError::Capacity); }
    if m.archive_digest!=digest_bytes(archive) { return Err(ArtifactError::Digest); }
    if signed.signer!=trusted.to_bytes() { return Err(ArtifactError::Signature); }
    let bytes=manifest_bytes(m)?;
    trusted.verify_strict(&bytes,&Signature::from_slice(&signed.signature).map_err(|_|ArtifactError::Signature)?).map_err(|_|ArtifactError::Signature)?;
    safe_relative(&m.command)?;
    if !m.files.get(&m.command).is_some_and(|f|f.executable) { return Err(ArtifactError::Invalid("command not executable".into())); }
    let mut total=0u64;
    for (path,file) in &m.files {
        safe_relative(path)?;
        if file.digest.len()!=64 || !file.digest.bytes().all(|c|c.is_ascii_hexdigit()) { return Err(ArtifactError::Digest); }
        total=total.checked_add(file.bytes).ok_or(ArtifactError::Capacity)?;
    }
    if total>limits.extracted_bytes || m.args.iter().any(|a|a.len()>4096 || a.contains('\0')) || m.env.iter().any(|(k,v)| k.is_empty() || k.starts_with("HYPERMIND_") || k.contains(['=','\0']) || k.len()>256 || v.len()>4096 || v.contains('\0')) || m.capabilities.iter().any(|(k,v)|validate_id(k).is_err() || *v==0) { return Err(ArtifactError::Invalid("manifest declarations".into())); }
    Ok(VerifiedArtifact{manifest:m.clone(),manifest_digest:digest_bytes(&bytes)})
}
pub fn extract_artifact(verified:&VerifiedArtifact, archive:&[u8], destination:&Path, limits:ArtifactLimits)->Result<(),ArtifactError> {
    if verified.manifest.archive_digest!=digest_bytes(archive) || archive.len() as u64>limits.archive_bytes { return Err(ArtifactError::Digest); }
    if destination.exists() { return Err(ArtifactError::Invalid("staging destination exists".into())); }
    fs::create_dir(destination)?;
    let result=extract_inner(verified,archive,destination,limits);
    if result.is_err() { let _=fs::remove_dir_all(destination); }
    result
}
fn extract_inner(verified:&VerifiedArtifact, archive:&[u8], destination:&Path, limits:ArtifactLimits)->Result<(),ArtifactError> {
    let mut tar=tar::Archive::new(archive);
    let mut seen=std::collections::BTreeSet::new(); let mut total=0u64; let mut count=0usize;
    for entry in tar.entries()? {
        let mut entry=entry?; count+=1; if count>limits.file_count { return Err(ArtifactError::Capacity); }
        if !entry.header().entry_type().is_file() { return Err(ArtifactError::Invalid("archive must contain regular files only".into())); }
        let path=entry.path()?.to_str().ok_or_else(||ArtifactError::Invalid("non UTF-8 path".into()))?.to_string(); safe_relative(&path)?;
        if !seen.insert(path.clone()) { return Err(ArtifactError::Invalid("duplicate archive path".into())); }
        let expected=verified.manifest.files.get(&path).ok_or_else(||ArtifactError::Invalid("undeclared archive path".into()))?;
        if entry.size()!=expected.bytes { return Err(ArtifactError::Digest); }
        total=total.checked_add(entry.size()).ok_or(ArtifactError::Capacity)?; if total>limits.extracted_bytes { return Err(ArtifactError::Capacity); }
        let target=destination.join(&path); if let Some(parent)=target.parent() { fs::create_dir_all(parent)?; }
        let mut bytes=Vec::new(); entry.by_ref().take(expected.bytes.saturating_add(1)).read_to_end(&mut bytes)?;
        if bytes.len() as u64!=expected.bytes || digest_bytes(&bytes)!=expected.digest { return Err(ArtifactError::Digest); }
        let mut file=OpenOptions::new().write(true).create_new(true).open(&target)?; file.write_all(&bytes)?; file.sync_all()?;
        #[cfg(unix)] { use std::os::unix::fs::PermissionsExt; fs::set_permissions(&target,fs::Permissions::from_mode(if expected.executable {0o755} else {0o644}))?; }
    }
    if seen.len()!=verified.manifest.files.len() { return Err(ArtifactError::Invalid("missing archive file".into())); }
    fs::File::open(destination)?.sync_all()?;
    Ok(())
}
