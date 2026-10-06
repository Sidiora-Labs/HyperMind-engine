use hm_context::Scope;
use hm_core::ActorId;
use hm_serve::{actor::{ActorConfig,ActorEngine},context_memory::{self,MemoryCommand,MemoryRecord,MemoryRequest,RecordKind},retention::{self,RetentionApproval,RetentionPolicy}};
use std::{fs,path::Path};
fn scope()->Scope {Scope{owner_id:"owner".into(),project_id:"retention-metrics".into(),workspace_id:None}}
async fn write(actor:&ActorEngine,id:&str,command:MemoryCommand) {
    context_memory::execute(actor,&scope(),&scope(),MemoryRequest{version:1,scope:scope(),request_id:id.into(),command}).await.unwrap();
}
fn bytes(path:&Path)->u64 {
    fs::read_dir(path).unwrap().map(|entry| {let entry=entry.unwrap();if entry.file_type().unwrap().is_dir(){bytes(&entry.path())}else{entry.metadata().unwrap().len()}}).sum()
}
async fn assert_active_bytes(actor:&ActorEngine,root:&Path) {
    let active=hm_ledger::retention::active_directory(root).unwrap();
    let actual=bytes(&active.join("log"));assert!(actual>0);assert_eq!(actor.stats().await.unwrap().log_bytes,actual);
}
async fn purge(actor:&ActorEngine,id:&str)->(RetentionApproval,retention::PurgeReceipt) {
    let plan=retention::plan(actor,&scope(),&scope(),vec![id.into()],actor.stats().await.unwrap().applied.last_lsn.get(),RetentionPolicy{revision:1,tombstone_grace_ns:0}).await.unwrap();
    let approval=retention::approve(&scope(),plan).unwrap();
    let receipt=retention::execute(actor,&scope(),approval.clone()).await.unwrap();(approval,receipt)
}
#[tokio::test]
async fn two_native_purges_have_per_plan_replay_counts_and_active_log_bytes_after_restart() {
    let directory=tempfile::tempdir().unwrap();
    let config=ActorConfig{actor_directory:directory.path().into(),actor:ActorId::new(83),user:[1;16],kek:[2;32],projection_map_bytes:16*1024*1024};
    let actor=ActorEngine::open(config.clone()).await.unwrap();
    for id in ["first","second"] {
        write(&actor,&format!("create-{id}"),MemoryCommand::Create{record:MemoryRecord::new(id,RecordKind::Note,format!("Private record {id}"),1)}).await;
    }
    let mut revised=MemoryRecord::new("second",RecordKind::Note,"Second corrected evidence",1);revised.revision=2;revised.revision_digest=revised.computed_revision_digest().unwrap();
    write(&actor,"revise-second",MemoryCommand::Revise{record:revised,expected_revision:1}).await;
    write(&actor,"tombstone-first",MemoryCommand::Tombstone{id:"first".into(),expected_revision:1}).await;
    write(&actor,"tombstone-second",MemoryCommand::Tombstone{id:"second".into(),expected_revision:2}).await;
    assert_active_bytes(&actor,directory.path()).await;
    let root=actor.verification_status().await.unwrap().root;
    let (first,first_receipt)=purge(&actor,"first").await;assert_eq!(first_receipt.redacted_frames,2);assert_eq!(first_receipt.redacted_frames,first.plan.affected_lsns.len());
    let retry=retention::execute(&actor,&scope(),first.clone()).await.unwrap();assert!(retry.replayed);assert_eq!(retry.redacted_frames,first_receipt.redacted_frames);let mut normalized=retry.clone();normalized.replayed=false;assert_eq!(serde_json::to_value(normalized).unwrap(),serde_json::to_value(&first_receipt).unwrap());
    assert_active_bytes(&actor,directory.path()).await;
    let (second,second_receipt)=purge(&actor,"second").await;assert_eq!(second_receipt.redacted_frames,3);assert_eq!(second_receipt.redacted_frames,second.plan.affected_lsns.len());
    let retry=retention::execute(&actor,&scope(),second.clone()).await.unwrap();assert!(retry.replayed);assert_eq!(retry.redacted_frames,second_receipt.redacted_frames);let mut normalized=retry.clone();normalized.replayed=false;assert_eq!(serde_json::to_value(normalized).unwrap(),serde_json::to_value(&second_receipt).unwrap());
    assert_active_bytes(&actor,directory.path()).await;assert_eq!(actor.verification_status().await.unwrap().root,root);
    let tail=actor.stats().await.unwrap().applied.last_lsn;
    let mut forged=second.clone();forged.plan.affected_lsns.push(999);assert!(retention::execute(&actor,&scope(),forged).await.is_err());assert_eq!(actor.stats().await.unwrap().applied.last_lsn,tail);
    actor.shutdown().await.unwrap();let actor=ActorEngine::open(config).await.unwrap();
    let retry=retention::execute(&actor,&scope(),second).await.unwrap();assert!(retry.replayed);assert_eq!(retry.redacted_frames,3);assert_eq!(retry.plan_digest,second_receipt.plan_digest);assert_eq!(retry.record_ids,second_receipt.record_ids);assert_eq!(retry.original_root,second_receipt.original_root);let mut normalized=retry.clone();normalized.replayed=false;assert_eq!(serde_json::to_value(normalized).unwrap(),serde_json::to_value(&second_receipt).unwrap());
    assert_active_bytes(&actor,directory.path()).await;assert_eq!(actor.verification_status().await.unwrap().root,root);
    let state=context_memory::rebuild(&actor,&scope()).await.unwrap();assert!(state.purged_records.contains_key("first"));assert!(state.purged_records.contains_key("second"));assert!(state.records.is_empty());
    actor.shutdown().await.unwrap();
}
