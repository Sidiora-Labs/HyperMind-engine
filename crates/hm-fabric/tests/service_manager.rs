#![cfg(target_os="linux")]
use ed25519_dalek::{Signer,SigningKey};
use hm_context::digest_bytes;
use hm_fabric::{Scope,artifacts::*,registry_install::*,supervisor::Probe,containment::{ContainmentPolicy,ResourceLimits,NetworkPolicy,WritableBind},service_manager::*};
use std::{collections::BTreeMap,fs,path::Path,time::{Duration,Instant},thread};
const WORKER:&str=r#"import os,sys,time,subprocess
if 'INVOCATION_ID' not in os.environ:
    time.sleep(300)
else:
    journal=os.environ['HM_SERVICE_JOURNAL']
    with open(journal+'/starts','a') as stream:
        stream.write('start\n');stream.flush();os.fsync(stream.fileno())
    count=len(open(journal+'/starts').readlines())
    if count==1:
        sys.exit(23)
    child=subprocess.Popen([sys.executable,'-c','import time;time.sleep(300)'])
    with open(journal+'/ready','w') as stream:
        stream.write(str(child.pid));stream.flush();os.fsync(stream.fileno())
    time.sleep(300)
"#;
fn grant(path:&Path){use std::os::unix::fs::PermissionsExt;fs::set_permissions(path,fs::Permissions::from_mode(0o755)).unwrap();}
fn policy(journal:&Path)->ContainmentPolicy{ContainmentPolicy{rootfs:"/".into(),writable_binds:vec![WritableBind{host_path:journal.into(),guest_path:journal.into()}],uid:61040,gid:61040,network:NetworkPolicy::DenyIp,limits:ResourceLimits{address_space_bytes:256*1024*1024,cpu_seconds:5,open_files:64,file_size_bytes:1024*1024,processes:64}}}
fn scope()->Scope{Scope{owner_id:"owner".into(),project_id:"service".into(),workspace_id:None}}
struct OwnedCleanup<'a>{manager:&'a LinuxServiceManager,receipt:ServiceReceipt}
impl Drop for OwnedCleanup<'_>{fn drop(&mut self){let _=self.manager.uninstall(&self.receipt,"owner");}}
#[test]
fn native_user_manager_signed_registration_restarts_and_drains_all_children(){
 if let Err(error)=LinuxServiceManager::availability(){assert!(matches!(error,ServiceError::Unavailable(_)));panic!("actual native user manager unavailable: {error}");}
 let dir=tempfile::tempdir().unwrap();grant(dir.path());let journal=dir.path().join("journal");fs::create_dir(&journal).unwrap();grant(&journal);std::os::unix::fs::chown(&journal,Some(61040),Some(61040)).unwrap();
 let key=SigningKey::from_bytes(&[67;32]);let files=BTreeMap::from([("bin/python3".to_string(),fs::read("/usr/bin/python3").unwrap()),("worker.py".to_string(),WORKER.as_bytes().to_vec())]);let mut archive=tar::Builder::new(Vec::new());for (path,bytes) in &files{let mut header=tar::Header::new_gnu();header.set_size(bytes.len() as u64);header.set_mode(if path=="worker.py"{0o644}else{0o755});header.set_cksum();archive.append_data(&mut header,path,bytes.as_slice()).unwrap();}let archive=archive.into_inner().unwrap();
 let manifest=ArtifactManifest{format_version:1,scope:scope(),module_id:"service-worker".into(),version:"1".into(),target_os:std::env::consts::OS.into(),target_arch:std::env::consts::ARCH.into(),protocol_version:1,archive_digest:digest_bytes(&archive),archive_bytes:archive.len() as u64,files:files.iter().map(|(path,bytes)|(path.clone(),ArtifactFile{digest:digest_bytes(bytes),bytes:bytes.len() as u64,executable:path!="worker.py"})).collect(),capabilities:BTreeMap::from([("background-work".into(),1)]),command:"bin/python3".into(),args:vec!["worker.py".into()],env:BTreeMap::from([("PYTHONHOME".into(),"/usr".into()),("HM_SERVICE_JOURNAL".into(),journal.to_str().unwrap().into())])};let signed=SignedArtifact{signer:key.verifying_key().to_bytes(),signature:key.sign(&manifest_bytes(&manifest).unwrap()).to_bytes().to_vec(),manifest};
 let registry_root=dir.path().join("registry");let registry=InstallRegistry::open(&registry_root,scope(),key.verifying_key(),ArtifactLimits::default()).unwrap();grant(&registry_root);let install=registry.preview_install(&signed,&archive,"owner",&signed.manifest.capabilities).unwrap();let launch=LaunchReview{readiness:Probe::ProcessAlive,health:Probe::ProcessAlive,readiness_timeout:Duration::from_secs(5),shutdown_timeout:Duration::from_millis(100),containment:policy(&journal)};let mut artifact=registry.apply_plan(&install,install.digest(),"owner",Some(&archive),launch,None).unwrap();artifact.process.shutdown().unwrap();
 let payload=registry_root.join("releases").join(&artifact.receipt.release).join("payload");let source=ServiceArtifact::from_registry(&registry,&artifact.receipt,payload).unwrap();let manager=LinuxServiceManager::open(dir.path().join("receipts"),scope(),key.verifying_key()).unwrap();let spec=ServiceSpec{service_key:format!("native-test-{}-{}",std::process::id(),std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()),artifact:source,containment:policy(&journal),reviewed_capabilities:signed.manifest.capabilities.clone(),restart_delay:Duration::from_millis(100),restart_limit:4,restart_window:Duration::from_secs(30),stop_timeout:Duration::from_millis(200)};
 let plan=manager.preview_service(&spec,"owner").unwrap();assert!(!plan.receipt().unit_path.exists());assert!(manager.install(&plan,"wrong-digest","owner").is_err());assert!(!plan.receipt().unit_path.exists());let _cleanup=OwnedCleanup{manager:&manager,receipt:plan.receipt().clone()};let receipt=manager.install(&plan,plan.digest(),"owner").unwrap();
 let deadline=Instant::now()+Duration::from_secs(15);loop{if journal.join("ready").exists(){break;}assert!(Instant::now()<deadline,"native restart did not reach actual worker readiness: {:?}",manager.status(&receipt));thread::sleep(Duration::from_millis(50));}
 let status=manager.status(&receipt).unwrap();assert_eq!(status.active_state,"active");assert!(status.restarts>=1);assert!(status.main_pid>0);assert!(!status.control_group.is_empty());assert!(fs::read_to_string(journal.join("starts")).unwrap().lines().count()>=2);let live_group=Path::new("/sys/fs/cgroup").join(status.control_group.trim_start_matches('/'));assert!(fs::read_to_string(live_group.join("cgroup.procs")).unwrap().lines().count()>=3,"actual worker descendant missing from native control group");
 let mut foreign=receipt.clone();foreign.owner="other".into();assert!(matches!(manager.stop(&foreign),Err(ServiceError::Ownership(_))));assert_eq!(manager.status(&receipt).unwrap().active_state,"active");let original=fs::read(&receipt.unit_path).unwrap();fs::write(&receipt.unit_path,b"[Service]\nExecStart=/bin/false\n").unwrap();assert!(matches!(manager.stop(&receipt),Err(ServiceError::Ownership(_))));fs::write(&receipt.unit_path,&original).unwrap();
 let stop=Instant::now();manager.stop(&receipt).unwrap();assert!(stop.elapsed()<Duration::from_secs(5));let stopped=manager.status(&receipt).unwrap();assert_eq!(stopped.main_pid,0);assert!(stopped.active_state=="inactive"||stopped.active_state=="failed","native stopped result: {stopped:?}");if stopped.active_state=="failed" {assert!(!stopped.native_result.is_empty());}println!("native stopped lifecycle: {stopped:?}");assert!(!Path::new(&format!("/proc/{}",status.main_pid)).exists());let group=Path::new("/sys/fs/cgroup").join(status.control_group.trim_start_matches('/'));if group.exists(){assert!(!fs::read_to_string(group.join("cgroup.events")).unwrap().contains("populated 1"));}
 manager.uninstall(&receipt,"owner").unwrap();assert!(!receipt.unit_path.exists());
}
