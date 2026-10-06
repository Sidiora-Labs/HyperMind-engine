use hm_context::{historian::{select_chunks_with_spans,ChunkLimits,HistorianClaim,HistorianResult,SummaryTier}, maintenance::{JobKind,JobLease,JobRequest,Usage}, notes::{Note,NoteKind,Predicate,NotesCommand,Grant,AttributeProposal}, *};
use hm_core::{ActorId,ConversationId};
use hm_schema::{event::{encode_event_envelope,CURRENT_SCHEMA_VERSION},events::{EventEnvelope,EventPayload,UserMsg,Retention,Sensitivity}};
use hm_serve::{actor::{ActorConfig,ActorEngine,IncomingEvent},context_jobs::{execute,published_summaries,ContextJobRequest,ContextJobAction},session_context::{activate,SessionContextRequest}};
use serde_json::Value;
use std::collections::{BTreeSet,BTreeMap};
fn scope()->Scope { Scope{owner_id:"owner".into(),project_id:"project".into(),workspace_id:None} }
fn config(path:&std::path::Path)->ActorConfig { ActorConfig{actor_directory:path.join("7"),actor:ActorId::new(7),user:[1;16],kek:[2;32],projection_map_bytes:16*1024*1024} }
fn request(id:&str,action:ContextJobAction)->ContextJobRequest { ContextJobRequest{version:1,scope:scope(),request_id:id.into(),action} }
async fn run(actor:&ActorEngine,id:&str,action:ContextJobAction)->Value { execute(actor,&scope(),"owner",request(id,action)).await.unwrap() }
fn note(kind:NoteKind)->Note { Note{id:"note".into(),kind,revision:1,text:"source-backed knowledge".into(),parents:BTreeSet::new(),contradictions:BTreeSet::new(),expires_at_ns:None,predicate:Predicate::True,tombstoned:false} }
async fn setup(actor:&ActorEngine) {
    actor.append(vec![IncomingEvent{kind:hm_ledger::frame::EventKind::UserMsg,conversation:ConversationId::derive("session"),payload:encode_event_envelope(&EventEnvelope{schema_version:CURRENT_SCHEMA_VERSION,payload:EventPayload::UserMsg(Box::new(UserMsg{content:b"Measured temperature is 20 C".to_vec()})),connection_id:None,client_seq:0,client_event_index:0,client_event_count:1,origin_actor:0,run_id:None,model_provenance:None,authority:hm_schema::events::Authority::UserAsserted,retention:Retention::Durable,sensitivity:Sensitivity::Personal,event_time_ns:0})}]).await.unwrap();
    activate(actor,"session",&scope(),SessionContextRequest{version:1,scope:scope(),session_id:"session".into(),budget:TokenBudget{context_tokens:4096,reserved_output_tokens:256,required_tokens:0},generation:1,model_id:"gpt-4o".into(),query:String::new(),required_message_ids:vec![],tier:Default::default(),defer_reductions:false,profile:Default::default()}).await.unwrap();
}
#[tokio::test]
async fn notes_grants_proposals_atomic_cas_dedup_and_restart() {
    let directory=tempfile::tempdir().unwrap();
    let actor=ActorEngine::open(config(directory.path())).await.unwrap();
    let create=request("create",ContextJobAction::Notes{command:NotesCommand::Create(note(NoteKind::Primer))});
    let first=execute(&actor,&scope(),"owner",create.clone()).await.unwrap();
    assert_eq!(first,execute(&actor,&scope(),"owner",create).await.unwrap());
    assert!(execute(&actor,&scope(),"owner",request("create",ContextJobAction::Inspect)).await.is_err());
    run(&actor,"grant",ContextJobAction::Notes{command:NotesCommand::SetGrant(Grant{principal:"reader".into(),note_id:"note".into(),read:true,write:false})}).await;
    let read=execute(&actor,&scope(),"reader",request("read",ContextJobAction::ReadNote{id:"note".into(),now_ns:0,facts:BTreeMap::new()})).await.unwrap();
    assert_eq!(read["text"],"source-backed knowledge");
    assert!(execute(&actor,&scope(),"stranger",request("read2",ContextJobAction::ReadNote{id:"note".into(),now_ns:0,facts:BTreeMap::new()})).await.is_err());
    run(&actor,"attribute-grant",ContextJobAction::Notes{command:NotesCommand::SetAttributeGrant{principal:"reader".into(),key:"preference".into(),write:true}}).await;
    execute(&actor,&scope(),"reader",request("proposal",ContextJobAction::Notes{command:NotesCommand::ProposeAttribute(AttributeProposal{id:"proposal".into(),key:"preference".into(),value:"brief".into(),base_revision:0,proposer:"reader".into()})})).await.unwrap();
    assert!(run(&actor,"attribute-before",ContextJobAction::ReadAttribute{key:"preference".into()}).await.is_null());
    assert!(execute(&actor,&scope(),"reader",request("wrong-accept",ContextJobAction::Notes{command:NotesCommand::AcceptAttribute{proposal_id:"proposal".into()}})).await.is_err());
    run(&actor,"accept",ContextJobAction::Notes{command:NotesCommand::AcceptAttribute{proposal_id:"proposal".into()}}).await;
    assert_eq!(run(&actor,"attribute-after",ContextJobAction::ReadAttribute{key:"preference".into()}).await,serde_json::json!([1,"brief"]));
    let mut anchor=note(NoteKind::Anchor);anchor.id="anchor".into();
    run(&actor,"anchor-create",ContextJobAction::Notes{command:NotesCommand::Create(anchor.clone())}).await;
    anchor.revision=2;
    assert!(execute(&actor,&scope(),"owner",request("anchor-revise",ContextJobAction::Notes{command:NotesCommand::Revise{note:anchor,expected_revision:1}})).await.is_err());
    let mut revised=note(NoteKind::Primer);revised.revision=2;revised.text="revised knowledge".into();
    let a=request("rev-a",ContextJobAction::Notes{command:NotesCommand::Revise{note:revised.clone(),expected_revision:1}});
    let b=request("rev-b",ContextJobAction::Notes{command:NotesCommand::Revise{note:revised,expected_revision:1}});
    let trusted=scope();let (a,b)=tokio::join!(execute(&actor,&trusted,"owner",a),execute(&actor,&trusted,"owner",b));
    assert_eq!(usize::from(a.is_ok())+usize::from(b.is_ok()),1);
    actor.shutdown().await.unwrap();
    let actor=ActorEngine::open(config(directory.path())).await.unwrap();
    let read=run(&actor,"read-restart",ContextJobAction::ReadNote{id:"note".into(),now_ns:0,facts:BTreeMap::new()}).await;
    assert_eq!(read["revision"],2);
    let mut foreign=request("foreign",ContextJobAction::Inspect);foreign.scope.owner_id="foreign".into();
    assert!(execute(&actor,&scope(),"owner",foreign).await.is_err());
    actor.shutdown().await.unwrap();
}
#[tokio::test]
async fn real_ledger_sources_historian_publication_and_maintenance_settlement() {
    let directory=tempfile::tempdir().unwrap();
    let actor=ActorEngine::open(config(directory.path())).await.unwrap();setup(&actor).await;
    let current=hm_serve::context_history::replay(&actor,&scope(),"session","session").await.unwrap().history;
    let sources=current.visible_messages().into_iter().cloned().collect::<Vec<_>>();
    let spans=sources.iter().map(|source|current.source_span(&source.id).unwrap()).collect::<Vec<_>>();
    let chunk=select_chunks_with_spans(&sources,&spans,ChunkLimits{max_messages:10,max_bytes:10000}).unwrap().remove(0);
    let id=run(&actor,"enqueue",ContextJobAction::HistorianEnqueue{session_id:"session".into(),cursor:current.cursor(),policy_revision:1,chunk:chunk.clone(),reservation:1_000_000,now_ms:u64::MAX}).await["result"].as_str().unwrap().to_string();
    let claim:HistorianClaim=serde_json::from_value(run(&actor,"claim",ContextJobAction::HistorianClaim{worker:"worker".into(),now_ms:u64::MAX,lease_ms:60_000}).await["result"].clone()).unwrap();
    let now=std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as u64;
    let hm_context::historian::JobState::Claimed{expires_at_ms,..}= &claim.job.state else { panic!("expected live claim") };
    assert!(*expires_at_ms > now && *expires_at_ms <= now+60_000);
    let input=JobRequest{kind:JobKind::Verification,sources:sources.clone(),cursor:current.cursor(),source_revision:current.cursor().sequence,policy_revision:1,reservation:100};
    let maintenance_id=run(&actor,"maint-enqueue",ContextJobAction::MaintenanceEnqueue{session_id:"session".into(),request:input}).await["result"].as_str().unwrap().to_string();
    assert!(run(&actor,"budget-blocked-running",ContextJobAction::MaintenanceClaim{now_ms:u64::MAX}).await["result"].is_null());
    run(&actor,"heartbeat",ContextJobAction::HistorianHeartbeat{claim:claim.clone(),now_ms:u64::MAX,lease_ms:60_000}).await;
    let result=HistorianResult{source_digest:chunk.digest.clone(),tiers:std::array::from_fn(|_|SummaryTier{text:"Measured temperature is 20 C".into(),coverage:spans.clone()})};
    let mut invalid=result.clone();invalid.source_digest=digest_bytes(b"wrong");
    assert!(execute(&actor,&scope(),"owner",request("wrong-result",ContextJobAction::HistorianComplete{claim:claim.clone(),result:invalid,usage:Usage::Known(123),now_ms:u64::MAX})).await.is_err());
    run(&actor,"complete",ContextJobAction::HistorianComplete{claim:claim.clone(),result,usage:Usage::Unknown,now_ms:u64::MAX}).await;
    assert_eq!(published_summaries(&actor,&scope(),"session").await.unwrap()[0].id,id);
    assert!(execute(&actor,&scope(),"owner",request("stale",ContextJobAction::HistorianFail{claim,usage:Usage::Known(0),now_ms:3,cooldown_ms:1})).await.is_err());
    let state=run(&actor,"after-historian-complete",ContextJobAction::Inspect).await;
    assert_eq!(state["maintenance"]["spent"],0);
    let historian_maintenance=state["historian_maintenance"][&id].as_str().unwrap().to_string();
    assert!(run(&actor,"budget-blocked-unknown",ContextJobAction::MaintenanceClaim{now_ms:u64::MAX}).await["result"].is_null());
    actor.shutdown().await.unwrap();
    let actor=ActorEngine::open(config(directory.path())).await.unwrap();
    let held=run(&actor,"inspect-held-restart",ContextJobAction::Inspect).await;
    assert_eq!(held["maintenance"]["unknown_usage"].as_object().unwrap().values().next().unwrap(),1_000_000);
    run(&actor,"hist-settle",ContextJobAction::MaintenanceSettle{id:historian_maintenance,attempt:1,actual:3}).await;
    let lease:JobLease=serde_json::from_value(run(&actor,"maint-claim",ContextJobAction::MaintenanceClaim{now_ms:0}).await["result"].clone()).unwrap();
    run(&actor,"maint-cancel",ContextJobAction::MaintenanceCancel{id:maintenance_id.clone(),now_ms:1}).await;
    assert!(execute(&actor,&scope(),"owner",request("maint-stale",ContextJobAction::MaintenanceComplete{lease:lease.clone(),usage:Usage::Known(0),output_digest:digest_bytes(b"result"),now_ms:2})).await.is_err());
    actor.shutdown().await.unwrap();
    let actor=ActorEngine::open(config(directory.path())).await.unwrap();
    run(&actor,"settle",ContextJobAction::MaintenanceSettle{id:maintenance_id,attempt:lease.attempt,actual:7}).await;
    let inspected=run(&actor,"inspect",ContextJobAction::Inspect).await;
    assert_eq!(inspected["maintenance"]["spent"],10);
    assert_eq!(published_summaries(&actor,&scope(),"session").await.unwrap().len(),1);
    actor.shutdown().await.unwrap();
}

#[test]
fn maintenance_frontier_never_skips_unfinished_registered_coverage() {
    use hm_context::maintenance::{MaintenanceScheduler,SchedulerConfig};
    let mut scheduler=MaintenanceScheduler::new(scope(),SchedulerConfig{max_concurrency:3,budget:100,lease_ms:100,backoff_ms:1,max_attempts:3}).unwrap();
    let mut ids=Vec::new();
    for sequence in 1..=3 {
        let mut source=SourceMessage{id:format!("source-{sequence}"),ordinal:sequence,role:MessageRole::User,parts:vec![MessagePart::Text{text:format!("Observation {sequence}")}],occurred_at_ns:None,recorded_at_ns:1,authority:Authority::UserAsserted,source_digest:String::new()};
        source.source_digest=source.computed_digest().unwrap();
        ids.push(scheduler.enqueue(JobRequest{kind:JobKind::Verification,sources:vec![source],cursor:Cursor{epoch:1,sequence},source_revision:sequence,policy_revision:1,reservation:10}).unwrap());
    }
    let a=scheduler.claim_job(&ids[0],0,100).unwrap().unwrap();
    let b=scheduler.claim_job(&ids[1],0,100).unwrap().unwrap();
    let c=scheduler.claim_job(&ids[2],0,100).unwrap().unwrap();
    scheduler.complete(&c,&c.fence,Usage::Known(1),&digest_bytes(b"third"),1).unwrap();
    assert!(!scheduler.snapshot().watermarks.contains_key(&JobKind::Verification));
    assert_eq!(scheduler.snapshot().watermark_gaps[&JobKind::Verification],vec![Cursor{epoch:1,sequence:1},Cursor{epoch:1,sequence:2}]);
    scheduler.complete(&a,&a.fence,Usage::Known(1),&digest_bytes(b"first"),1).unwrap();
    assert_eq!(scheduler.snapshot().watermarks[&JobKind::Verification],Cursor{epoch:1,sequence:1});
    scheduler.complete(&b,&b.fence,Usage::Known(1),&digest_bytes(b"second"),1).unwrap();
    assert_eq!(scheduler.snapshot().watermarks[&JobKind::Verification],Cursor{epoch:1,sequence:3});
    assert!(scheduler.snapshot().watermark_gaps[&JobKind::Verification].is_empty());
}

#[tokio::test]
async fn runtime_expiration_and_cancellation_hold_historian_budget() {
    let directory=tempfile::tempdir().unwrap();
    let actor=ActorEngine::open(config(directory.path())).await.unwrap();setup(&actor).await;
    let current=hm_serve::context_history::replay(&actor,&scope(),"session","session").await.unwrap().history;
    let sources=current.visible_messages().into_iter().cloned().collect::<Vec<_>>();
    let spans=sources.iter().map(|source|current.source_span(&source.id).unwrap()).collect::<Vec<_>>();
    let chunk=select_chunks_with_spans(&sources,&spans,ChunkLimits{max_messages:10,max_bytes:10000}).unwrap().remove(0);
    let id=run(&actor,"cancel-enqueue",ContextJobAction::HistorianEnqueue{session_id:"session".into(),cursor:current.cursor(),policy_revision:1,chunk:chunk.clone(),reservation:100,now_ms:u64::MAX}).await["result"].as_str().unwrap().to_string();
    let claim:HistorianClaim=serde_json::from_value(run(&actor,"cancel-claim",ContextJobAction::HistorianClaim{worker:"worker".into(),now_ms:u64::MAX,lease_ms:60_000}).await["result"].clone()).unwrap();
    run(&actor,"cancel-historian",ContextJobAction::HistorianCancel{id:id.clone()}).await;
    let state=run(&actor,"cancel-state",ContextJobAction::Inspect).await;
    let budget_id=state["historian_maintenance"][&id].as_str().unwrap().to_string();
    assert_eq!(state["maintenance"]["unknown_usage"].as_object().unwrap().values().next().unwrap(),100);
    run(&actor,"cancel-settle",ContextJobAction::MaintenanceSettle{id:budget_id,attempt:claim.attempt,actual:11}).await;
    run(&actor,"policy-next",ContextJobAction::SetPolicy{session_id:"session".into(),expected_revision:1,revision:2}).await;
    let id=run(&actor,"expire-enqueue",ContextJobAction::HistorianEnqueue{session_id:"session".into(),cursor:current.cursor(),policy_revision:2,chunk:chunk.clone(),reservation:100,now_ms:0}).await["result"].as_str().unwrap().to_string();
    let claim:HistorianClaim=serde_json::from_value(run(&actor,"expire-claim",ContextJobAction::HistorianClaim{worker:"worker".into(),now_ms:0,lease_ms:1}).await["result"].clone()).unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    run(&actor,"expire",ContextJobAction::HistorianExpire{now_ms:0,cooldown_ms:0}).await;
    let result=HistorianResult{source_digest:chunk.digest.clone(),tiers:std::array::from_fn(|_|SummaryTier{text:"Measured temperature is 20 C".into(),coverage:spans.clone()})};
    assert!(execute(&actor,&scope(),"owner",request("expire-late-result",ContextJobAction::HistorianComplete{claim,result,usage:Usage::Known(0),now_ms:0})).await.is_err());
    let state=run(&actor,"expired-state",ContextJobAction::Inspect).await;
    assert_eq!(state["maintenance"]["spent"],11);
    let budget_id=state["historian_maintenance"][&id].as_str().unwrap();
    assert_eq!(state["maintenance"]["unknown_usage"][format!("{budget_id}:1")],100);
    assert!(run(&actor,"future-cannot-bypass-cooldown",ContextJobAction::HistorianClaim{worker:"worker".into(),now_ms:u64::MAX,lease_ms:60_000}).await["result"].is_null());
    actor.shutdown().await.unwrap();
}
