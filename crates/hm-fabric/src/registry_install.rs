use crate::{artifacts::*, supervisor::{self, ManagedProcess, Probe, ProcessSpec, RestartPolicy}, Scope};
use crate::containment::ContainmentPolicy;
use ed25519_dalek::VerifyingKey;
use fs2::FileExt;
use hm_context::{digest_bytes,validate_id};
use serde::{Deserialize,Serialize};
use std::{collections::BTreeMap,fs::{self,File,OpenOptions},io::{Read,Write},path::{Path,PathBuf},time::Duration};

#[derive(Debug,thiserror::Error)]
pub enum InstallError {
 #[error(transparent)] Artifact(#[from]ArtifactError),
 #[error("installation I/O: {0}")] Io(#[from]std::io::Error),
 #[error("installation metadata: {0}")] Json(#[from]serde_json::Error),
 #[error("plan or ownership changed")] Conflict,
 #[error("capabilities not reviewed")] Review,
 #[error("candidate process: {0}")] Process(#[from]supervisor::SupervisorError),
}
#[derive(Clone,Debug,Eq,PartialEq,Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OwnershipReceipt {
 pub scope:Scope,pub module_id:String,pub owner:String,pub generation:u64,pub launch_id:String,
 pub manifest_digest:String,pub release:String,pub files:BTreeMap<String,ArtifactFile>,
}
#[derive(Clone,Debug,Eq,PartialEq,Serialize,Deserialize)]
pub enum RemovalPolicy { Retain, Export{destination:PathBuf}, Purge }
#[derive(Clone,Debug,Serialize,Deserialize)]
enum Action { Install{signed:SignedArtifact}, Rollback{receipt:OwnershipReceipt}, Uninstall{receipt:OwnershipReceipt,owned:Vec<OwnershipReceipt>,policy:RemovalPolicy} }
#[derive(Clone,Debug,Serialize,Deserialize)]
pub struct InstallPlan { action:Action,owner:String,state_digest:String,reviewed:BTreeMap<String,u32>,digest:String }
impl InstallPlan {
 pub fn digest(&self)->&str { &self.digest }
 pub fn capabilities(&self)->&BTreeMap<String,u32> { &self.reviewed }
}
#[derive(Clone,Debug,Default,Serialize,Deserialize)]
struct Slot { active:Option<OwnershipReceipt>,previous:Option<OwnershipReceipt> }
#[derive(Clone,Debug,Default,Serialize,Deserialize)]
struct State { generation:u64,modules:BTreeMap<String,Slot>,owned:BTreeMap<String,OwnershipReceipt> }
pub struct InstallRegistry { root:PathBuf,scope:Scope,trusted:VerifyingKey,limits:ArtifactLimits }
pub struct Activation { pub receipt:OwnershipReceipt,pub process:ManagedProcess,pub incumbent_shutdown_error:Option<String> }
#[derive(Clone,Debug)]
pub struct RemovalReceipt { pub ownership:OwnershipReceipt,pub policy:RemovalPolicy,pub retained_path:Option<PathBuf> }
pub struct LaunchReview { pub readiness:Probe,pub health:Probe,pub readiness_timeout:Duration,pub shutdown_timeout:Duration,pub containment:ContainmentPolicy }
impl InstallRegistry {
 pub fn open(root:impl AsRef<Path>,scope:Scope,trusted:VerifyingKey,limits:ArtifactLimits)->Result<Self,InstallError> {
  scope.validate().map_err(|_|InstallError::Conflict)?;
  if root.as_ref().exists() && fs::symlink_metadata(root.as_ref())?.file_type().is_symlink() { return Err(InstallError::Conflict); }
  fs::create_dir_all(root.as_ref())?; let root=fs::canonicalize(root)?;
  let releases=root.join("releases"); if releases.exists() && fs::symlink_metadata(&releases)?.file_type().is_symlink() { return Err(InstallError::Conflict); }
  fs::create_dir_all(&releases)?;
  let this=Self{root,scope,trusted,limits}; let _lock=this.lock()?;
  let binding=this.root.join("registry.json");
  let expected=serde_json::to_vec(&(1u32,&this.scope,this.trusted.to_bytes()))?;
  if binding.exists() { if fs::read(binding)?!=expected { return Err(InstallError::Conflict); } }
  else { this.atomic_write("registry.json",&expected)?; }
  Ok(this)
 }
 fn lock(&self)->Result<File,InstallError> { let path=self.root.join("writer.lock"); if path.exists() && fs::symlink_metadata(&path)?.file_type().is_symlink() { return Err(InstallError::Conflict); } let file=OpenOptions::new().read(true).write(true).create(true).truncate(false).open(self.root.join("writer.lock"))?; file.lock_exclusive()?; Ok(file) }
 fn state(&self)->Result<State,InstallError> { let path=self.root.join("state.json"); if path.exists() { if fs::symlink_metadata(&path)?.file_type().is_symlink() || fs::metadata(&path)?.len()>4*1024*1024 { return Err(InstallError::Conflict); } Ok(serde_json::from_slice(&fs::read(path)?)?) } else { Ok(State::default()) } }
 fn state_digest(state:&State)->Result<String,InstallError> { Ok(digest_bytes(&serde_json::to_vec(state)?)) }
 fn atomic_write(&self,name:&str,bytes:&[u8])->Result<(),InstallError> {
  if bytes.len()>4*1024*1024 { return Err(InstallError::Conflict); }
  let destination=self.root.join(name); if destination.exists() && fs::symlink_metadata(&destination)?.file_type().is_symlink() { return Err(InstallError::Conflict); }
  let tmp=self.root.join(format!(".{name}.pending"));
  if tmp.exists() { if fs::symlink_metadata(&tmp)?.file_type().is_symlink() { return Err(InstallError::Conflict); } fs::remove_file(&tmp)?; }
  let mut file=OpenOptions::new().create_new(true).write(true).open(&tmp)?; file.write_all(bytes)?; file.sync_all()?;
  fs::rename(tmp,self.root.join(name))?; File::open(&self.root)?.sync_all()?; Ok(())
 }
 fn plan(&self,action:Action,owner:&str,state:&State,reviewed:BTreeMap<String,u32>)->Result<InstallPlan,InstallError> {
  validate_id(owner).map_err(|_|InstallError::Conflict)?;
  let mut plan=InstallPlan{action,owner:owner.into(),state_digest:Self::state_digest(state)?,reviewed,digest:String::new()};
  plan.digest=digest_bytes(&serde_json::to_vec(&plan)?); Ok(plan)
 }
 pub fn preview_install(&self,signed:&SignedArtifact,archive:&[u8],owner:&str,reviewed:&BTreeMap<String,u32>)->Result<InstallPlan,InstallError> {
  let verified=verify_artifact(signed,&self.trusted,&self.scope,archive,self.limits)?;
  if reviewed!=&verified.manifest().capabilities { return Err(InstallError::Review); }
  let _lock=self.lock()?; let state=self.state()?;
  if let Some(active)=state.modules.get(&signed.manifest.module_id).and_then(|s|s.active.as_ref()) { if active.owner!=owner { return Err(InstallError::Conflict); } }
  self.plan(Action::Install{signed:signed.clone()},owner,&state,reviewed.clone())
 }
 pub fn preview_update(&self,signed:&SignedArtifact,archive:&[u8],owner:&str,reviewed:&BTreeMap<String,u32>)->Result<InstallPlan,InstallError> {
  let plan=self.preview_install(signed,archive,owner,reviewed)?;
  let _lock=self.lock()?; if !self.state()?.modules.get(&signed.manifest.module_id).is_some_and(|s|s.active.is_some()) { return Err(InstallError::Conflict); } Ok(plan)
 }
 pub fn preview_rollback(&self,module:&str,owner:&str)->Result<InstallPlan,InstallError> {
  let _lock=self.lock()?; let state=self.state()?;
  let slot=state.modules.get(module).ok_or(InstallError::Conflict)?;
  if !slot.active.as_ref().is_some_and(|r|r.owner==owner) { return Err(InstallError::Conflict); }
  let receipt=slot.previous.clone().filter(|r|r.owner==owner).ok_or(InstallError::Conflict)?;
  self.verify_owned(&receipt)?;
  let signed=self.signed_release(&receipt)?;
  self.plan(Action::Rollback{receipt},owner,&state,signed.manifest.capabilities)
 }
 pub fn preview_uninstall(&self,module:&str,owner:&str,policy:RemovalPolicy)->Result<InstallPlan,InstallError> {
  let _lock=self.lock()?; let state=self.state()?;
  let receipt=state.modules.get(module).and_then(|s|s.active.clone()).filter(|r|r.owner==owner).ok_or(InstallError::Conflict)?;
  self.verify_owned(&receipt)?;
  if let RemovalPolicy::Export{destination}=&policy { if destination.exists() || !destination.is_absolute() || destination.starts_with(&self.root) || fs::canonicalize(destination.parent().ok_or(InstallError::Conflict)?)?.starts_with(&self.root) { return Err(InstallError::Conflict); } }
  let owned=state.owned.values().filter(|r|r.module_id==module && r.owner==owner).cloned().collect::<Vec<_>>();
  for item in &owned { self.verify_owned(item)?; }
  self.plan(Action::Uninstall{receipt,owned,policy},owner,&state,BTreeMap::new())
 }
 fn check_plan(&self,plan:&InstallPlan,expected_digest:&str,owner:&str,state:&State)->Result<(),InstallError> {
  let mut canonical=plan.clone(); canonical.digest.clear();
  if plan.owner!=owner || plan.digest!=expected_digest || digest_bytes(&serde_json::to_vec(&canonical)?)!=expected_digest || plan.state_digest!=Self::state_digest(state)? { return Err(InstallError::Conflict); }
  Ok(())
 }
 fn release_path(&self,receipt:&OwnershipReceipt)->Result<PathBuf,InstallError> {
  if receipt.scope!=self.scope || receipt.release!=receipt.manifest_digest || receipt.release.len()!=64 || !receipt.release.bytes().all(|b|b.is_ascii_hexdigit()) { return Err(InstallError::Conflict); }
  Ok(self.root.join("releases").join(&receipt.release))
 }
 fn signed_release(&self,receipt:&OwnershipReceipt)->Result<SignedArtifact,InstallError> {
  let path=self.release_path(receipt)?.join("signed.json"); if fs::metadata(&path)?.len()>1024*1024 { return Err(InstallError::Conflict); }
  let signed:SignedArtifact=serde_json::from_slice(&fs::read(path)?)?;
  let bytes=manifest_bytes(&signed.manifest)?;
  if digest_bytes(&bytes)!=receipt.manifest_digest || signed.signer!=self.trusted.to_bytes() || signed.manifest.files!=receipt.files || signed.manifest.module_id!=receipt.module_id || signed.manifest.scope!=receipt.scope { return Err(InstallError::Conflict); }
  self.trusted.verify_strict(&bytes,&ed25519_dalek::Signature::from_slice(&signed.signature).map_err(|_|ArtifactError::Signature)?).map_err(|_|ArtifactError::Signature)?;
  Ok(signed)
 }
 fn verify_owned(&self,receipt:&OwnershipReceipt)->Result<(),InstallError> {
  self.signed_release(receipt)?;
  let release=self.release_path(receipt)?; let payload=release.join("payload");
  if fs::symlink_metadata(&release)?.file_type().is_symlink() || fs::symlink_metadata(&payload)?.file_type().is_symlink() { return Err(InstallError::Conflict); }
  let mut found=BTreeMap::new(); self.scan_owned(&payload,&payload,&mut found)?;
  if found!=receipt.files { return Err(InstallError::Conflict); }
  let names=fs::read_dir(&release)?.map(|r|r.map(|e|e.file_name())).collect::<Result<Vec<_>,_>>()?;
  if names.len()!=2 || !names.iter().all(|n|n=="payload" || n=="signed.json") { return Err(InstallError::Conflict); }
  Ok(())
 }
 fn scan_owned(&self,base:&Path,path:&Path,found:&mut BTreeMap<String,ArtifactFile>)->Result<(),InstallError> {
  for entry in fs::read_dir(path)? {
   let entry=entry?; let meta=fs::symlink_metadata(entry.path())?;
   if meta.file_type().is_symlink() { return Err(InstallError::Conflict); }
   if meta.is_dir() { let before=found.len(); self.scan_owned(base,&entry.path(),found)?; if found.len()==before { return Err(InstallError::Conflict); } }
   else if meta.is_file() {
    if found.len()>=self.limits.file_count || meta.len()>self.limits.extracted_bytes { return Err(InstallError::Conflict); }
    let mut data=Vec::new(); File::open(entry.path())?.take(self.limits.extracted_bytes+1).read_to_end(&mut data)?;
    #[cfg(unix)] let executable={use std::os::unix::fs::PermissionsExt; let mode=meta.permissions().mode() & 0o7777; if mode!=0o755 && mode!=0o644 { return Err(InstallError::Conflict); } mode==0o755};
    #[cfg(not(unix))] let executable=false;
    let relative=entry.path().strip_prefix(base).map_err(|_|InstallError::Conflict)?.to_str().ok_or(InstallError::Conflict)?.to_string();
    found.insert(relative,ArtifactFile{digest:digest_bytes(&data),bytes:data.len() as u64,executable});
   } else { return Err(InstallError::Conflict); }
  } Ok(())
 }
 pub fn apply_plan(&self,plan:&InstallPlan,expected_digest:&str,owner:&str,archive:Option<&[u8]>,launch:LaunchReview,mut incumbent:Option<&mut ManagedProcess>)->Result<Activation,InstallError> {
  let _lock=self.lock()?; let mut state=self.state()?; self.check_plan(plan,expected_digest,owner,&state)?;
  let generation=state.generation.checked_add(1).ok_or(InstallError::Conflict)?;
  let (signed,mut receipt)=match &plan.action {
   Action::Install{signed}=>{
    let archive=archive.ok_or(InstallError::Conflict)?;
    let verified=verify_artifact(signed,&self.trusted,&self.scope,archive,self.limits)?;
    if verified.manifest().capabilities!=plan.reviewed { return Err(InstallError::Review); }
    let receipt=OwnershipReceipt{scope:self.scope.clone(),module_id:signed.manifest.module_id.clone(),owner:owner.into(),generation,launch_id:String::new(),manifest_digest:verified.digest().into(),release:verified.digest().into(),files:signed.manifest.files.clone()};
    if state.owned.get(&receipt.manifest_digest).is_some_and(|r|r.owner!=owner || r.module_id!=receipt.module_id) { return Err(InstallError::Conflict); }
    let final_path=self.release_path(&receipt)?;
    if !final_path.exists() {
     let stage=self.root.join("releases").join(format!(".stage-{}",verified.digest()));
     if stage.exists() { return Err(InstallError::Conflict); } fs::create_dir(&stage)?;
     if let Err(error)=extract_artifact(&verified,archive,&stage.join("payload"),self.limits) { let _=fs::remove_dir(&stage); return Err(error.into()); }
     let mut file=OpenOptions::new().write(true).create_new(true).open(stage.join("signed.json"))?; file.write_all(&serde_json::to_vec(signed)?)?; file.sync_all()?;
     File::open(&stage)?.sync_all()?; fs::rename(&stage,&final_path)?; File::open(self.root.join("releases"))?.sync_all()?;
    }
    self.verify_owned(&receipt)?; (signed.clone(),receipt)
   },
   Action::Rollback{receipt}=>{ self.verify_owned(receipt)?; (self.signed_release(receipt)?,receipt.clone()) },
   Action::Uninstall{..}=>return Err(InstallError::Conflict),
  };
  receipt.generation=generation;
  let has_active=state.modules.get(&receipt.module_id).is_some_and(|s|s.active.is_some());
  if has_active!=incumbent.is_some() { return Err(InstallError::Conflict); }
  if let Some(old)=incumbent.as_mut() { let current=state.modules.get(&receipt.module_id).and_then(|s|s.active.as_ref()).ok_or(InstallError::Conflict)?; if old.identity.module_id!=receipt.module_id || old.identity.generation!=current.generation || old.identity.launch_id!=current.launch_id || !old.healthy()? { return Err(InstallError::Conflict); } }
  let payload=self.release_path(&receipt)?.join("payload");
  let spec=ProcessSpec{module_id:receipt.module_id.clone(),command:payload.join(&signed.manifest.command),args:signed.manifest.args.clone(),env:signed.manifest.env.clone(),cwd:payload,readiness:launch.readiness,health:launch.health,readiness_timeout:launch.readiness_timeout,shutdown_timeout:launch.shutdown_timeout,stderr_bytes:16384,drain_message:None,restart:RestartPolicy{max_restarts:0,initial_backoff:Duration::ZERO,max_backoff:Duration::ZERO}};
  let mut candidate=supervisor::launch_candidate(spec,generation,launch.containment)?;
  if !candidate.healthy()? { return Err(InstallError::Conflict); }
  receipt.launch_id=candidate.identity.launch_id.clone();
  state.owned.insert(receipt.manifest_digest.clone(),receipt.clone());
  let slot=state.modules.entry(receipt.module_id.clone()).or_default(); slot.previous=slot.active.replace(receipt.clone()); state.generation=generation;
  self.atomic_write("state.json",&serde_json::to_vec(&state)?)?;
  let incumbent_shutdown_error=incumbent.and_then(|old|old.shutdown().err().map(|e|e.to_string()));
  Ok(Activation{receipt,process:candidate,incumbent_shutdown_error})
 }
 pub fn apply_uninstall(&self,plan:&InstallPlan,expected_digest:&str,owner:&str,mut process:Option<&mut ManagedProcess>)->Result<RemovalReceipt,InstallError> {
  let _lock=self.lock()?; let mut state=self.state()?; self.check_plan(plan,expected_digest,owner,&state)?;
  let Action::Uninstall{receipt,owned,policy}=&plan.action else { return Err(InstallError::Conflict); };
  if process.is_none() { return Err(InstallError::Conflict); }
  self.verify_owned(receipt)?;
  for item in owned { self.verify_owned(item)?; }
  let release=self.release_path(receipt)?;
  if let Some(child)=process.as_mut() { if child.identity.module_id!=receipt.module_id || child.identity.generation!=receipt.generation || child.identity.launch_id!=receipt.launch_id { return Err(InstallError::Conflict); } }
  let retained_path=match policy {
   RemovalPolicy::Retain=>Some(release.clone()),
   RemovalPolicy::Export{destination}=>{
    if destination.exists() || destination.starts_with(&self.root) || fs::canonicalize(destination.parent().ok_or(InstallError::Conflict)?)?.starts_with(&self.root) { return Err(InstallError::Conflict); }
    fs::create_dir(destination)?;
    let result=(||->Result<(),InstallError>{ for item in owned { let source=self.release_path(item)?; let exported=destination.join(&item.release); fs::create_dir(&exported)?; for (path,_) in &item.files { let target=exported.join("payload").join(path); fs::create_dir_all(target.parent().ok_or(InstallError::Conflict)?)?; fs::copy(source.join("payload").join(path),&target)?; File::open(target)?.sync_all()?; } fs::copy(source.join("signed.json"),exported.join("signed.json"))?; File::open(exported.join("signed.json"))?.sync_all()?; File::open(&exported)?.sync_all()?; } let mut receipt_file=File::create(destination.join("ownership.json"))?; receipt_file.write_all(&serde_json::to_vec(owned)?)?; receipt_file.sync_all()?; File::open(destination)?.sync_all()?; Ok(())})();
    if let Err(error)=result { let _=fs::remove_dir_all(destination); return Err(error); } Some(destination.clone())
   },
   RemovalPolicy::Purge=>None,
  };
  if let Some(child)=process.as_mut() { child.shutdown()?; }
  state.modules.remove(&receipt.module_id);
  if matches!(policy,RemovalPolicy::Purge) { for item in owned { state.owned.remove(&item.manifest_digest); } }
  state.generation=state.generation.checked_add(1).ok_or(InstallError::Conflict)?;
  self.atomic_write("state.json",&serde_json::to_vec(&state)?)?;
  if matches!(policy,RemovalPolicy::Purge) { for item in owned { self.verify_owned(item)?; fs::remove_dir_all(self.release_path(item)?)?; } File::open(self.root.join("releases"))?.sync_all()?; }
  Ok(RemovalReceipt{ownership:receipt.clone(),policy:policy.clone(),retained_path})
 }
 pub fn reconcile_registry(&self,candidate_generation:u64)->Result<Vec<OwnershipReceipt>,InstallError> {
  let _lock=self.lock()?; let state=self.state()?; if state.generation!=candidate_generation { return Err(InstallError::Conflict); }
  let mut receipts=Vec::new(); for slot in state.modules.values() { if let Some(receipt)=&slot.active { self.verify_owned(receipt)?; receipts.push(receipt.clone()); } } Ok(receipts)
 }
 pub fn installed(&self,module:&str)->Result<Option<OwnershipReceipt>,InstallError> { let _lock=self.lock()?; Ok(self.state()?.modules.get(module).and_then(|s|s.active.clone())) }
}
