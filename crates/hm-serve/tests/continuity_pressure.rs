use hm_context::{continuity_pressure::*, reduction::{ReductionItem,ReductionPolicy}, historian::{select_chunks_with_spans,ChunkLimits,HistorianResult,SummaryTier}, history::SourceHistory, provider::CapabilityProfile, types::*};
use hm_core::ActorId;
use hm_serve::{actor::{ActorConfig,ActorEngine},context_projection::*};
async fn tail(a:&ActorEngine)->hm_core::LSN {a.stats().await.unwrap().applied.last_lsn}
#[tokio::test]
async fn pressure_hook_fences_and_native_deferred_projection() {
    let dir=tempfile::tempdir().unwrap();
    let config=ActorConfig{actor_directory:dir.path().into(),actor:ActorId::new(71),user:[1;16],kek:[2;32],projection_map_bytes:128*1024*1024};
    let actor=ActorEngine::open(config.clone()).await.unwrap();
    let scope=Scope{owner_id:"owner".into(),project_id:"project".into(),workspace_id:None};
    let mut history=SourceHistory::new(scope.clone(),"pressure-session").unwrap();
    for n in 0..12 {
        let text=if n==10 {"measurement output ".repeat(4000)} else {format!("Work record {n}: measured temperature remained stable. ").repeat(80)};
        let parts=if n==9 {vec![MessagePart::ToolCall{call_id:"measurement".into(),name:"sensor".into(),arguments:"{}".into()}]} else if n==10 {vec![MessagePart::ToolResult{call_id:"measurement".into(),content:text.clone(),failed:false}]} else {vec![MessagePart::Text{text:text.clone()}]};
        let mut message=SourceMessage{id:format!("record-{n}"),ordinal:n+1,role:if n==9 {MessageRole::Assistant} else if n==10 {MessageRole::Tool} else {MessageRole::User},parts,occurred_at_ns:None,recorded_at_ns:0,authority:Authority::UserAsserted,source_digest:String::new()};
        message.source_digest=message.computed_digest().unwrap();history.ingest(message,text.into_bytes()).unwrap();
    }
    let request=ProjectionRequest{model_id:"gpt-4o".into(),policy_revision:"policy".into(),permission_revision:"grant".into(),required_blocks:vec![],required_message_ids:vec!["record-11".into()],tier:SummaryLevel::Detailed,defer_reductions:true};
    let first=assemble(&actor,&history,request.clone(),tail(&actor).await).await.unwrap();
    let sources:Vec<_>=history.visible_messages().into_iter().take(9).cloned().collect();
    let spans:Vec<_>=sources.iter().map(|m|history.source_span(&m.id).unwrap()).collect();
    let chunk=select_chunks_with_spans(&sources,&spans,ChunkLimits{max_messages:20,max_bytes:100000}).unwrap().remove(0);
    let result=HistorianResult{source_digest:chunk.digest.clone(),tiers:["Nine chronological measurements show stable temperature, with no reported deviations.","Nine stable temperature measurements.","Temperature stable.","Stable."].map(|text|SummaryTier{text:text.into(),coverage:spans.clone()})};
    let queued=publish_summary(&actor,&history,&first.fence,chunk,result.clone(),tail(&actor).await).await.unwrap();
    assert_eq!(queued.bytes,first.bytes);assert_eq!(queued.pending_reductions,1);
    actor.shutdown().await.unwrap();let actor=ActorEngine::open(config).await.unwrap();
    let deferred=assemble(&actor,&history,request.clone(),tail(&actor).await).await.unwrap();assert_eq!(deferred.bytes,first.bytes);
    let mut fold=request.clone();fold.defer_reductions=false;
    let folded=assemble(&actor,&history,fold.clone(),tail(&actor).await).await.unwrap();assert_eq!(folded.generation,first.generation+1);
    let event_count=actor.stats().await.unwrap().log_events;
    assert_eq!(assemble(&actor,&history,fold.clone(),tail(&actor).await).await.unwrap().generation,folded.generation);assert_eq!(event_count,actor.stats().await.unwrap().log_events);
    for (n,tier) in [SummaryLevel::Detailed,SummaryLevel::Condensed,SummaryLevel::Brief,SummaryLevel::Outline].into_iter().enumerate(){fold.tier=tier;let view=assemble(&actor,&history,fold.clone(),tail(&actor).await).await.unwrap();assert_eq!(view.blocks[0].text,result.tiers[n].text);}
    let messages:Vec<_>=history.visible_messages().into_iter().cloned().collect();
    let source_spans=messages.iter().map(|m|(m.id.clone(),history.source_span(&m.id).unwrap())).collect();
    let items=messages.iter().map(|m|{let text=String::from_utf8(history.recover(&scope,&history.source_span(&m.id).unwrap()).unwrap()).unwrap();ReductionItem::original(ContextBlock{id:m.id.clone(),tokens:text.len() as u64,text,authority:m.authority,provenance:vec![history.source_span(&m.id).unwrap()],required:false})}).collect();
    let snapshot=PressureSnapshot{fence:HookFence{cache:folded.fence.clone(),generation:folded.generation,grant_revision:"grant".into(),profile_revision:"profile".into()},cursor:Cursor::default(),messages,source_spans,items,policy:ReductionPolicy::default()};
    let counter=|bytes:&[u8]|Ok(bytes.len() as u64);
    let budget=TokenBudget{context_tokens:9000,reserved_output_tokens:1000,required_tokens:0};
    let plan=plan_pressure(&snapshot,&["record-11".into()],budget,CapabilityProfile::default(),&counter).unwrap();assert!(plan.rendered.report.token_count<=8000);assert!(plan.reduction.selections.iter().any(|s|s.id=="record-11"&&s.protected));
    assert!(plan.reduction.omitted.iter().any(|o|o.id=="record-10"));assert!(plan.reduction.omitted.iter().any(|o|o.id=="record-9"));
    assert_eq!(expand(&actor,&scope,"pressure-session",&history.source_span("record-10").unwrap()).await.unwrap(),b"measurement output ".repeat(4000));
    let before=pre_hook_fence(&snapshot).unwrap();assert!(post_hook_receipt(&before,&before,&plan,CapabilityProfile::default(),budget).is_ok());
    for field in 0..5 {let mut after=before.clone();match field {0=>after.generation+=1,1=>after.cache.source_revision="changed".into(),2=>after.grant_revision="changed".into(),3=>after.profile_revision="changed".into(),_=>after.cache.policy_revision="changed".into()};assert!(post_hook_receipt(&before,&after,&plan,CapabilityProfile::default(),budget).is_err());}
    assert!(plan_pressure(&snapshot,&["record-10".into()],budget,CapabilityProfile::default(),&counter).is_err());
    actor.shutdown().await.unwrap();
}
