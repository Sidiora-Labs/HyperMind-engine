use hm_context::{cache::SafetyInvalidation, historian::{select_chunks_with_spans, ChunkLimits, HistorianResult, SummaryTier}, history::{SourceHistory, SourceRelation}, types::*};
use hm_core::ActorId;
use hm_serve::{actor::{ActorConfig, ActorEngine}, context_projection::*};
async fn tail(actor:&ActorEngine)->hm_core::LSN { actor.stats().await.unwrap().applied.last_lsn }
fn scope() -> Scope { Scope { owner_id: "owner".into(), project_id: "project".into(), workspace_id: None } }
fn config(path: &std::path::Path) -> ActorConfig { ActorConfig { actor_directory: path.to_owned(), actor: ActorId::new(19), user: [1;16], kek:[2;32], projection_map_bytes:16*1024*1024 } }
fn request() -> ProjectionRequest { ProjectionRequest { model_id:"gpt-4o".into(), policy_revision:"policy-1".into(), permission_revision:"permission-1".into(), required_blocks:vec![], required_message_ids:vec![], tier:SummaryLevel::Detailed, defer_reductions:false } }
fn history() -> SourceHistory {
    let mut h=SourceHistory::new(scope(),"session").unwrap();
    for n in 0..3 { let text=format!("Original message {n} with preserved source bytes."); let mut m=SourceMessage {id:format!("source-{n}"),ordinal:n+1,role:MessageRole::User,parts:vec![MessagePart::Text{text:text.clone()}],occurred_at_ns:None,recorded_at_ns:0,authority:Authority::UserAsserted,source_digest:String::new()};m.source_digest=m.computed_digest().unwrap(); h.ingest(m,text.into_bytes()).unwrap(); }
    h
}
fn summary(h:&SourceHistory) -> (hm_context::historian::SourceChunk,HistorianResult) {
    let sources:Vec<_>=h.visible_messages().into_iter().take(2).cloned().collect();
    let spans:Vec<_>=sources.iter().map(|m|h.source_span(&m.id).unwrap()).collect();
    let chunk=select_chunks_with_spans(&sources,&spans,ChunkLimits{max_messages:10,max_bytes:10000}).unwrap().remove(0);
    let tiers=["Detailed chronological summary.","Condensed summary.","Brief summary.","Outline."] .map(|text|SummaryTier{text:text.into(),coverage:spans.clone()});
    let result=HistorianResult{source_digest:chunk.digest.clone(),tiers}; (chunk,result)
}
#[tokio::test]
async fn persisted_summary_reduction_defers_exact_bytes_and_survives_restart_with_expansion() {
    let directory=tempfile::tempdir().unwrap(); let engine=ActorEngine::open(config(directory.path())).await.unwrap(); let h=history(); let r=request();
    let first=assemble(&engine,&h,r.clone(),tail(&engine).await).await.unwrap(); assert_eq!(first.generation,1); assert_eq!(first.messages.len(),3);
    let events=engine.stats().await.unwrap().log_events;
    let replay=assemble(&engine,&h,r.clone(),tail(&engine).await).await.unwrap(); assert!(replay.replayed); assert_eq!(first.bytes,replay.bytes); assert_eq!(events,engine.stats().await.unwrap().log_events);
    let (chunk,result)=summary(&h);
    let queued=publish_summary(&engine,&h,&first.fence,chunk.clone(),result.clone(),tail(&engine).await).await.unwrap(); assert_eq!(queued.bytes,first.bytes); assert_eq!(queued.pending_reductions,1);
    let events=engine.stats().await.unwrap().log_events;
    publish_summary(&engine,&h,&first.fence,chunk,result,tail(&engine).await).await.unwrap(); assert_eq!(events,engine.stats().await.unwrap().log_events);
    let mut deferred=r.clone();deferred.defer_reductions=true;
    assert_eq!(assemble(&engine,&h,deferred,tail(&engine).await).await.unwrap().bytes,first.bytes);
    engine.shutdown().await.unwrap();
    let engine=ActorEngine::open(config(directory.path())).await.unwrap();
    let restored=current(&engine,&scope(),"session").await.unwrap().unwrap(); assert_eq!(restored.bytes,first.bytes); assert_eq!(restored.pending_reductions,1);
    let folded=assemble(&engine,&h,r.clone(),tail(&engine).await).await.unwrap(); assert_eq!(folded.generation,2); assert_eq!(folded.messages.len(),1); assert_eq!(folded.blocks[0].authority,Authority::DerivedInference); assert_eq!(folded.blocks[0].provenance,vec![h.source_span("source-0").unwrap(),h.source_span("source-1").unwrap()]);
    assert_eq!(expand(&engine,&scope(),"session",&folded.coverage[0]).await.unwrap(),b"Original message 0 with preserved source bytes.");
    engine.shutdown().await.unwrap();
    let engine=ActorEngine::open(config(directory.path())).await.unwrap(); assert_eq!(assemble(&engine,&h,r,tail(&engine).await).await.unwrap().bytes,folded.bytes); engine.shutdown().await.unwrap();
}
#[tokio::test]
async fn all_four_tiers_are_selected_with_exact_original_coverage_and_required_protection() {
    let directory=tempfile::tempdir().unwrap();let engine=ActorEngine::open(config(directory.path())).await.unwrap();let h=history();let mut r=request();let first=assemble(&engine,&h,r.clone(),tail(&engine).await).await.unwrap();let(chunk,result)=summary(&h);publish_summary(&engine,&h,&first.fence,chunk,result.clone(),tail(&engine).await).await.unwrap();
    for (index,tier) in [SummaryLevel::Detailed,SummaryLevel::Condensed,SummaryLevel::Brief,SummaryLevel::Outline].into_iter().enumerate() { r.tier=tier;let view=assemble(&engine,&h,r.clone(),tail(&engine).await).await.unwrap();assert_eq!(view.blocks[0].text,result.tiers[index].text);assert_eq!(view.blocks[0].provenance,result.tiers[index].coverage);assert_eq!(view.coverage.len(),3); }
    r.required_message_ids=vec!["source-0".into()];let view=assemble(&engine,&h,r,tail(&engine).await).await.unwrap();assert_eq!(view.messages.len(),3);assert!(view.blocks.is_empty());engine.shutdown().await.unwrap();
}
#[tokio::test]
async fn corrected_sources_permissions_and_required_bindings_invalidate_before_rebuild() {
    let directory=tempfile::tempdir().unwrap();let engine=ActorEngine::open(config(directory.path())).await.unwrap();let mut h=history();let mut r=request();let first=assemble(&engine,&h,r.clone(),tail(&engine).await).await.unwrap();let(chunk,result)=summary(&h);publish_summary(&engine,&h,&first.fence,chunk.clone(),result.clone(),tail(&engine).await).await.unwrap();assemble(&engine,&h,r.clone(),tail(&engine).await).await.unwrap();
    r.permission_revision="permission-2".into();let rebuilt=assemble(&engine,&h,r.clone(),tail(&engine).await).await.unwrap();assert!(rebuilt.generation>first.generation);assert!(rebuilt.blocks.is_empty());assert_eq!(rebuilt.messages.len(),3);
    assert!(publish_summary(&engine,&h,&first.fence,chunk.clone(),result.clone(),tail(&engine).await).await.is_err());
    r.required_blocks=vec![ContextBlock{id:"binding".into(),text:"Current binding".into(),authority:Authority::RuntimeFact,provenance:vec![],tokens:1,required:true}];let bound=assemble(&engine,&h,r.clone(),tail(&engine).await).await.unwrap();assert_eq!(bound.blocks[0].text,"Current binding");
    r.required_blocks[0].text="Corrected binding".into();let corrected=assemble(&engine,&h,r.clone(),tail(&engine).await).await.unwrap();assert!(corrected.generation>bound.generation);assert_eq!(corrected.blocks[0].text,"Corrected binding");
    h.relate(SourceRelation::Tombstone{id:"removed".into(),source_id:"source-0".into()}).unwrap();let changed=assemble(&engine,&h,r.clone(),tail(&engine).await).await.unwrap();assert!(changed.messages.iter().all(|m|m.id!="source-0"));assert!(publish_summary(&engine,&h,&changed.fence,chunk,result,tail(&engine).await).await.is_err());
    invalidate(&engine,&scope(),"session",SafetyInvalidation::AccessRevoked).await.unwrap();assert!(current(&engine,&scope(),"session").await.is_err());assert!(expand(&engine,&scope(),"session",&h.source_span("source-1").unwrap()).await.is_err());
    engine.shutdown().await.unwrap();let engine=ActorEngine::open(config(directory.path())).await.unwrap();assert!(current(&engine,&scope(),"session").await.is_err());assert!(assemble(&engine,&h,r.clone(),tail(&engine).await).await.is_err());r.permission_revision="permission-3".into();assert!(assemble(&engine,&h,r,tail(&engine).await).await.is_ok());engine.shutdown().await.unwrap();
}
#[tokio::test]
async fn wrong_coverage_and_scopes_refuse_without_receipt() {
    let directory=tempfile::tempdir().unwrap();let engine=ActorEngine::open(config(directory.path())).await.unwrap();let h=history();let first=assemble(&engine,&h,request(),tail(&engine).await).await.unwrap();let(mut chunk,mut result)=summary(&h);let before=engine.stats().await.unwrap().log_events;
    chunk.spans[0].byte_end+=1;chunk.digest=digest_bytes(&serde_json::to_vec(&(&chunk.sources,&chunk.spans)).unwrap());result.source_digest=chunk.digest.clone();for tier in &mut result.tiers {tier.coverage=chunk.spans.clone();}
    assert!(publish_summary(&engine,&h,&first.fence,chunk,result,tail(&engine).await).await.is_err());assert_eq!(before,engine.stats().await.unwrap().log_events);
    let mut foreign=scope();foreign.owner_id="other".into();assert!(current(&engine,&foreign,"session").await.unwrap().is_none());assert!(expand(&engine,&foreign,"session",&h.source_span("source-0").unwrap()).await.is_err());engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn cancelled_publication_is_removed_at_a_persisted_reduction_boundary() {
    let directory=tempfile::tempdir().unwrap();let engine=ActorEngine::open(config(directory.path())).await.unwrap();let h=history();let r=request();let first=assemble(&engine,&h,r.clone(),tail(&engine).await).await.unwrap();let(chunk,result)=summary(&h);
    sync_summaries(&engine,&h,&first.fence,vec![(chunk,result)],tail(&engine).await).await.unwrap();let summarized=assemble(&engine,&h,r.clone(),tail(&engine).await).await.unwrap();assert_eq!(summarized.messages.len(),1);
    let queued=sync_summaries(&engine,&h,&summarized.fence,vec![],tail(&engine).await).await.unwrap();assert_eq!(queued.bytes,summarized.bytes);
    engine.shutdown().await.unwrap();let engine=ActorEngine::open(config(directory.path())).await.unwrap();let removed=assemble(&engine,&h,r,tail(&engine).await).await.unwrap();assert_eq!(removed.messages.len(),3);assert!(removed.blocks.is_empty());engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn native_append_after_authoritative_source_read_refuses_stale_publication() {
    use hm_serve::{actor::IncomingEvent,context_history::{SourceIngestion,ingest,replay}};
    use hm_schema::{event::{self,CURRENT_SCHEMA_VERSION},events::{EventEnvelope,EventPayload,UserMsg,Retention,Sensitivity}};
    let directory=tempfile::tempdir().unwrap();let engine=ActorEngine::open(config(directory.path())).await.unwrap();
    let source=history().message("source-0").unwrap().clone();
    ingest(&engine,&scope(),&SourceIngestion{version:1,scope:scope(),session_id:"session".into(),conversation:"conversation".into(),message:source,original_bytes:b"Original message 0 with preserved source bytes.".to_vec()}).await.unwrap();
    let captured_tail=tail(&engine).await;
    let authoritative=replay(&engine,&scope(),"session","conversation").await.unwrap().history;
    let native=engine.clone();
    tokio::spawn(async move { native.append(vec![IncomingEvent {kind:hm_ledger::frame::EventKind::UserMsg,conversation:hm_core::ConversationId::derive("conversation"),payload:event::encode_event_envelope(&EventEnvelope{schema_version:CURRENT_SCHEMA_VERSION,payload:EventPayload::UserMsg(Box::new(UserMsg{content:b"Concurrent source evidence".to_vec()})),connection_id:None,client_seq:0,client_event_index:0,client_event_count:1,origin_actor:0,run_id:None,model_provenance:None,authority:hm_schema::events::Authority::RuntimeFact,retention:Retention::Durable,sensitivity:Sensitivity::Personal,event_time_ns:0})}]).await.unwrap(); }).await.unwrap();
    let events=engine.stats().await.unwrap().log_events;
    let stale=assemble(&engine,&authoritative,request(),captured_tail).await.unwrap_err();
    assert_eq!(stale.code,hm_core::ErrorCode::SequenceViolation);assert_eq!(stale.lsn,tail(&engine).await);
    assert_eq!(events,engine.stats().await.unwrap().log_events);
    assert!(current(&engine,&scope(),"session").await.unwrap().is_none());
    let fresh_tail=tail(&engine).await;let fresh=replay(&engine,&scope(),"session","conversation").await.unwrap().history;
    let published=assemble(&engine,&fresh,request(),fresh_tail).await.unwrap();
    assert_eq!(published.ledger_tail,tail(&engine).await.get());
    assert!(published.ledger_tail>captured_tail.get());assert_eq!(published.messages.len(),2);
    engine.shutdown().await.unwrap();
}
