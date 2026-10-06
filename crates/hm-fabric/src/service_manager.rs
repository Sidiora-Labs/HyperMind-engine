use crate::{artifacts::{ArtifactManifest,SignedArtifact},containment::ContainmentPolicy,registry_install::{InstallRegistry,OwnershipReceipt},Scope};
use ed25519_dalek::VerifyingKey;
use hm_context::{digest_bytes,validate_id};
use serde::{Deserialize,Serialize};
use std::{collections::BTreeMap,fs,path::PathBuf,time::Duration};
#[cfg(target_os="linux")]
pub mod linux;
#[cfg(target_os="linux")]
pub use linux::LinuxServiceManager;
#[derive(Debug,thiserror::Error)]
pub enum ServiceError {
 #[error("native service manager unavailable: {0}")] Unavailable(String),
 #[error("service ownership refused: {0}")] Ownership(String),
 #[error("native service operation refused: {0}")] Native(String),
 #[error("service operation timed out: {0}")] Timeout(String),
 #[error("service I/O: {0}")] Io(#[from]std::io::Error),
 #[error("service metadata: {0}")] Json(#[from]serde_json::Error),
 #[error("invalid service declaration: {0}")] Invalid(String),
}
#[derive(Clone,Debug)]
pub struct ServiceArtifact { signed:SignedArtifact,ownership:OwnershipReceipt,payload_root:PathBuf }
impl ServiceArtifact {
 pub fn from_registry(registry:&InstallRegistry,receipt:&OwnershipReceipt,payload_root:PathBuf)->Result<Self,ServiceError> {
  let installed=registry.installed(&receipt.module_id).map_err(|e|ServiceError::Ownership(e.to_string()))?;
  if installed.as_ref()!=Some(receipt) { return Err(ServiceError::Ownership("installed receipt changed".into())); }
  registry.reconcile_registry(receipt.generation).map_err(|e|ServiceError::Ownership(e.to_string()))?;
  if !payload_root.is_absolute() || fs::canonicalize(&payload_root)?!=payload_root || payload_root.file_name().is_none_or(|n|n!="payload") || payload_root.parent().and_then(|p|p.file_name()).is_none_or(|n|n!=receipt.release.as_str()) { return Err(ServiceError::Ownership("artifact release path mismatch".into())); }
  let metadata=payload_root.parent().ok_or_else(||ServiceError::Invalid("missing release".into()))?.join("signed.json"); if fs::symlink_metadata(&metadata)?.file_type().is_symlink() || fs::metadata(&metadata)?.len()>1024*1024 { return Err(ServiceError::Ownership("signed manifest file invalid".into())); }
  let signed:SignedArtifact=serde_json::from_slice(&fs::read(metadata)?)?;
  Ok(Self{signed,ownership:receipt.clone(),payload_root})
 }
 pub fn manifest(&self)->&ArtifactManifest { &self.signed.manifest }
 pub fn ownership(&self)->&OwnershipReceipt { &self.ownership }
 fn verify(&self,scope:&Scope,trusted:&VerifyingKey)->Result<(),ServiceError> {
  let m=&self.signed.manifest;
  let bytes=serde_json::to_vec(m)?;
  if &self.ownership.scope!=scope || &m.scope!=scope || m.module_id!=self.ownership.module_id || m.files!=self.ownership.files || digest_bytes(&bytes)!=self.ownership.manifest_digest || self.signed.signer!=trusted.to_bytes() { return Err(ServiceError::Ownership("signed artifact identity changed".into())); }
  trusted.verify_strict(&bytes,&ed25519_dalek::Signature::from_slice(&self.signed.signature).map_err(|_|ServiceError::Ownership("invalid artifact signature".into()))?).map_err(|_|ServiceError::Ownership("untrusted artifact signature".into()))?;
  for (path,expected) in &m.files {
   crate::artifacts::safe_relative(path).map_err(|e|ServiceError::Invalid(e.to_string()))?;
   let source=self.payload_root.join(path); let actual=fs::symlink_metadata(&source)?;
   if !actual.is_file() || actual.len()!=expected.bytes || fs::canonicalize(&source)?!=source || digest_bytes(&fs::read(&source)?)!=expected.digest { return Err(ServiceError::Ownership("artifact bytes or path changed".into())); }
   #[cfg(unix)] {use std::os::unix::fs::PermissionsExt; if actual.permissions().mode() & 0o7777 != if expected.executable {0o755} else {0o644} { return Err(ServiceError::Ownership("artifact permissions changed".into())); }}
  }
  Ok(())
 }
}
#[derive(Clone,Debug)]
pub struct ServiceSpec {
 pub service_key:String,pub artifact:ServiceArtifact,pub containment:ContainmentPolicy,
 pub reviewed_capabilities:BTreeMap<String,u32>,pub restart_delay:Duration,pub restart_limit:u32,pub restart_window:Duration,pub stop_timeout:Duration,
}
#[derive(Clone,Debug,Eq,PartialEq,Serialize,Deserialize)]
pub struct ServiceReceipt {
 pub scope:Scope,pub owner:String,pub service_name:String,pub unit_path:PathBuf,pub unit_digest:String,
 pub artifact:OwnershipReceipt,pub backend:String,
}
#[derive(Clone,Debug)]
pub struct ServicePlan { receipt:ServiceReceipt,unit:String,digest:String,artifact:ServiceArtifact }
impl ServicePlan {pub fn digest(&self)->&str {&self.digest} pub fn receipt(&self)->&ServiceReceipt{&self.receipt} pub fn native_unit(&self)->&str{&self.unit}}
#[derive(Clone,Debug,Eq,PartialEq)]
pub struct ServiceStatus {pub active_state:String,pub sub_state:String,pub main_pid:u32,pub restarts:u64,pub control_group:String,pub native_result:String}
pub fn unavailable_platform()->ServiceError {ServiceError::Unavailable(format!("native {} service manager is not qualified",std::env::consts::OS))}
fn service_name(scope:&Scope,owner:&str,key:&str)->Result<String,ServiceError> {
 validate_id(owner).map_err(|_|ServiceError::Invalid("invalid owner".into()))?;validate_id(key).map_err(|_|ServiceError::Invalid("invalid service key".into()))?;
 let digest=digest_bytes(&serde_json::to_vec(&(scope,owner,key))?);Ok(format!("hypermind-{}.service",&digest[..40]))
}
