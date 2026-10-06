#![cfg(target_os="linux")]
use ed25519_dalek::{Signer,SigningKey};
use hm_context::digest_bytes;
use hm_fabric::{Scope,artifacts::*,registry_install::*,supervisor::Probe,containment::{ContainmentPolicy,ResourceLimits,NetworkPolicy}};
use std::{collections::BTreeMap,fs,path::Path,time::Duration,net::TcpListener};
fn scope()->Scope {Scope{owner_id:"owner".into(),project_id:"project".into(),workspace_id:None}}
fn archive(path:&str,bytes:&[u8])->Vec<u8> {
 let mut builder=tar::Builder::new(Vec::new()); let mut header=tar::Header::new_gnu(); header.set_size(bytes.len() as u64);header.set_mode(0o755);header.set_cksum();builder.append_data(&mut header,path,bytes).unwrap();builder.into_inner().unwrap()
}
fn signed(version:&str,bytes:&[u8],archive:&[u8])->SignedArtifact {
 let manifest=ArtifactManifest{format_version:1,scope:scope(),module_id:"worker".into(),version:version.into(),target_os:std::env::consts::OS.into(),target_arch:std::env::consts::ARCH.into(),protocol_version:1,archive_digest:digest_bytes(archive),archive_bytes:archive.len() as u64,files:BTreeMap::from([("bin/sleep".into(),ArtifactFile{digest:digest_bytes(bytes),bytes:bytes.len() as u64,executable:true})]),capabilities:BTreeMap::from([("wait".into(),1)]),command:"bin/sleep".into(),args:vec!["30".into()],env:BTreeMap::new()};
 let key=SigningKey::from_bytes(&[42;32]);let signature=key.sign(&manifest_bytes(&manifest).unwrap()).to_bytes().to_vec();SignedArtifact{manifest,signer:key.verifying_key().to_bytes(),signature}
}
fn launch()->LaunchReview {
 LaunchReview{readiness:Probe::ProcessAlive,health:Probe::ProcessAlive,readiness_timeout:Duration::from_secs(2),shutdown_timeout:Duration::from_millis(20),containment:ContainmentPolicy{rootfs:"/".into(),writable_binds:vec![],uid:65534,gid:65534,network:NetworkPolicy::DenyIp,limits:ResourceLimits{address_space_bytes:128*1024*1024,cpu_seconds:5,open_files:64,file_size_bytes:1024*1024,processes:16}}}
}
fn grant(path:&Path) { use std::os::unix::fs::PermissionsExt;fs::set_permissions(path,fs::Permissions::from_mode(0o755)).unwrap(); }
#[test]
fn signed_archives_are_bounded_exact_and_escape_safe() {
 let bytes=fs::read("/bin/sleep").unwrap();let data=archive("bin/sleep",&bytes);let package=signed("1",&bytes,&data);let key=SigningKey::from_bytes(&[42;32]).verifying_key();let limits=ArtifactLimits::default();
 let verified=verify_artifact(&package,&key,&scope(),&data,limits).unwrap();let dir=tempfile::tempdir().unwrap();extract_artifact(&verified,&data,&dir.path().join("good"),limits).unwrap();assert_eq!(fs::read(dir.path().join("good/bin/sleep")).unwrap(),bytes);
 let mut bad=package.clone();bad.manifest.capabilities.insert("extra".into(),1);assert!(matches!(verify_artifact(&bad,&key,&scope(),&data,limits),Err(ArtifactError::Signature)));
 assert!(matches!(verify_artifact(&package,&key,&scope(),&data,ArtifactLimits{archive_bytes:1,..limits}),Err(ArtifactError::Capacity)));
 let mut escape=data.clone();escape[..100].fill(0);escape[..10].copy_from_slice(b"../escaped");escape[148..156].fill(b' ');let checksum=escape[..512].iter().map(|b|*b as u64).sum::<u64>();escape[148..156].copy_from_slice(format!("{checksum:06o}\0 ").as_bytes());let malicious=signed("2",&bytes,&escape);let verified=verify_artifact(&malicious,&key,&scope(),&escape,limits).unwrap();assert!(extract_artifact(&verified,&escape,&dir.path().join("bad"),limits).is_err());assert!(!dir.path().join("escaped").exists());assert!(!dir.path().join("bad").exists());
 let mut builder=tar::Builder::new(Vec::new());let mut header=tar::Header::new_gnu();header.set_entry_type(tar::EntryType::Symlink);header.set_size(0);header.set_mode(0o777);header.set_link_name("/etc/passwd").unwrap();header.set_cksum();builder.append_data(&mut header,"bin/sleep",std::io::empty()).unwrap();let link=builder.into_inner().unwrap();let malicious=signed("3",&bytes,&link);let verified=verify_artifact(&malicious,&key,&scope(),&link,limits).unwrap();assert!(extract_artifact(&verified,&link,&dir.path().join("link"),limits).is_err());
}
#[test]
fn real_candidate_update_failure_rollback_and_owned_purge() {
 let dir=tempfile::tempdir().unwrap();grant(dir.path());let root=dir.path().join("registry");let key=SigningKey::from_bytes(&[42;32]);let registry=InstallRegistry::open(&root,scope(),key.verifying_key(),ArtifactLimits::default()).unwrap();grant(&root);
 let bytes=fs::read("/bin/sleep").unwrap();let data=archive("bin/sleep",&bytes);let first=signed("1",&bytes,&data);let caps=first.manifest.capabilities.clone();
 assert!(registry.preview_install(&first,&data,"owner",&BTreeMap::new()).is_err());
 let plan=registry.preview_install(&first,&data,"owner",&caps).unwrap();let mut active=registry.apply_plan(&plan,plan.digest(),"owner",Some(&data),launch(),None).unwrap();assert!(active.process.healthy().unwrap());assert_eq!(registry.installed("worker").unwrap(),Some(active.receipt.clone()));
 assert!(registry.apply_plan(&plan,plan.digest(),"owner",Some(&data),launch(),None).is_err());
 let second=signed("2",&bytes,&data);let plan=registry.preview_update(&second,&data,"owner",&caps).unwrap();let socket=TcpListener::bind("127.0.0.1:0").unwrap();let closed=socket.local_addr().unwrap();drop(socket);let mut failure=launch();failure.readiness=Probe::Tcp(closed);failure.readiness_timeout=Duration::from_millis(80);
 assert!(registry.apply_plan(&plan,plan.digest(),"owner",Some(&data),failure,Some(&mut active.process)).is_err());assert!(active.process.healthy().unwrap());assert_eq!(registry.installed("worker").unwrap(),Some(active.receipt.clone()));
 let mut updated=registry.apply_plan(&plan,plan.digest(),"owner",Some(&data),launch(),Some(&mut active.process)).unwrap();assert!(updated.process.healthy().unwrap());assert!(active.process.healthy().unwrap()==false);
 let rollback=registry.preview_rollback("worker","owner").unwrap();let mut restored=registry.apply_plan(&rollback,rollback.digest(),"owner",None,launch(),Some(&mut updated.process)).unwrap();assert_eq!(restored.receipt.manifest_digest,active.receipt.manifest_digest);assert!(restored.process.healthy().unwrap());
 assert!(registry.preview_uninstall("worker","other",RemovalPolicy::Purge).is_err());let plan=registry.preview_uninstall("worker","owner",RemovalPolicy::Purge).unwrap();let owned=root.join("releases").join(&restored.receipt.release).join("payload/bin/sleep");let extra=owned.parent().unwrap().join("unowned");fs::write(&extra,b"unowned").unwrap();assert!(registry.apply_uninstall(&plan,plan.digest(),"owner",Some(&mut restored.process)).is_err());assert!(restored.process.healthy().unwrap());assert_eq!(fs::read(&extra).unwrap(),b"unowned");fs::remove_file(extra).unwrap();
 registry.apply_uninstall(&plan,plan.digest(),"owner",Some(&mut restored.process)).unwrap();assert!(registry.installed("worker").unwrap().is_none());assert!(!root.join("releases").join(&active.receipt.release).exists());assert!(!root.join("releases").join(&updated.receipt.release).exists());
}
#[test]
fn retain_and_export_preserve_owned_bytes() {
 for export in [false,true] {
  let dir=tempfile::tempdir().unwrap();grant(dir.path());let root=dir.path().join("registry");let registry=InstallRegistry::open(&root,scope(),SigningKey::from_bytes(&[42;32]).verifying_key(),ArtifactLimits::default()).unwrap();grant(&root);let bytes=fs::read("/bin/sleep").unwrap();let data=archive("bin/sleep",&bytes);let package=signed("1",&bytes,&data);let plan=registry.preview_install(&package,&data,"owner",&package.manifest.capabilities).unwrap();let mut activation=registry.apply_plan(&plan,plan.digest(),"owner",Some(&data),launch(),None).unwrap();let destination=dir.path().join("export");let policy=if export {RemovalPolicy::Export{destination:destination.clone()}} else {RemovalPolicy::Retain};let plan=registry.preview_uninstall("worker","owner",policy).unwrap();let removal=registry.apply_uninstall(&plan,plan.digest(),"owner",Some(&mut activation.process)).unwrap();assert!(registry.installed("worker").unwrap().is_none());let retained=removal.retained_path.unwrap();let path=if export {retained.join(&activation.receipt.release).join("payload/bin/sleep")} else {retained.join("payload/bin/sleep")};assert_eq!(fs::read(path).unwrap(),bytes);if export {assert!(destination.join("ownership.json").exists());}
 }
}
