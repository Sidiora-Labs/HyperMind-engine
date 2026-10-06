use hm_context::{Scope, temporal::IdentityRegistry};
use hm_core::ActorId;
use hm_serve::{actor::{ActorConfig,ActorEngine}, context_history, hypermid_import::{decode_context_export,import_batch}, hypermid_context_import::ContextSourceMaterial};
use std::{path::Path,process::Command};

fn config(path:&Path)->ActorConfig{ActorConfig{actor_directory:path.into(),actor:ActorId::new(29),user:[6;16],kek:[7;32],projection_map_bytes:16*1024*1024}}
fn produced()->serde_json::Value{
    let public_root=std::env::var("HYPERMID_PUBLIC_ROOT").expect("HYPERMID_PUBLIC_ROOT must identify an actual public source checkout for the export producer");
    let dir=tempfile::tempdir().unwrap();std::fs::create_dir(dir.path().join("src")).unwrap();
    let manifest=format!("[package]\nname='context-export-producer'\nversion='0.0.0'\nedition='2021'\n[dependencies]\nhypermid-context={{path={:?}}}\nhypermid-core={{path={:?}}}\nhypermid-contracts={{path={:?}}}\nserde_json='1'\n",format!("{public_root}/crates/hypermid-context"),format!("{public_root}/crates/hypermid-core"),format!("{public_root}/crates/hypermid-contracts"));
    std::fs::write(dir.path().join("Cargo.toml"),manifest).unwrap();
    std::fs::write(dir.path().join("src/main.rs"),r#"
use hypermid_context::export::{ContextExportBuilder,ContextSessionBinding,PortabilityEntryKind};
use hypermid_contracts::{Cursor,Digest,Id,Scope};
use hypermid_core::history::ContextItem;
use serde_json::json;
fn main(){
 let scope=Scope::new(Id::new("owner").unwrap(),Id::new("project").unwrap(),None);
 let binding=ContextSessionBinding{scope:scope.clone(),session_id:Id::new("imported-session").unwrap(),cursor:Cursor::new(3,2).unwrap()};
 let mut builder=ContextExportBuilder::new(Id::new("public-export").unwrap(),binding,"2026-10-06T12:00:00Z").unwrap();
 let mut sources=Vec::new();let mut first_digest=None;
 for (index,text) in ["Original observation.","Correction recorded."].iter().enumerate(){
  let item_id=format!("item-{index}");let source_id=format!("source-{index}");
  let mut bytes=serde_json::to_vec(&json!({"id":source_id,"text":text})).unwrap();bytes.extend_from_slice(b"\r\n");
  let source_digest=Digest::sha256(&bytes);
  let relations=if index==0{vec![]}else{vec![json!({"kind":"supersedes","item_id":"item-0","source_digest":first_digest})]};
  let value=json!({"item_id":item_id,"source_event_id":source_id,"source_digest":source_digest,"scope":scope,"session_id":"imported-session","cursor":{"epoch":3,"sequence":index+1},"role":"user","parts":[{"part_id":format!("part-{index}"),"kind":"text","text":text,"content_digest":Digest::sha256(text.as_bytes())}],"relations":relations,"created_at":"2026-10-06T12:00:00.123456789Z","recoverable":true});
  let item:ContextItem=serde_json::from_value(value).unwrap();item.validate_shape().unwrap();
  builder.push(Id::new(format!("reference-{index}")).unwrap(),PortabilityEntryKind::SourceReference,json!({"scope":scope,"session_id":"imported-session","item_id":item_id,"source_event_id":source_id,"source_digest":source_digest,"cursor":{"epoch":3,"sequence":index+1}})).unwrap();
  sources.push(json!({"version":1,"scope":scope,"session_id":"imported-session","item":serde_json::to_value(item).unwrap(),"original_bytes":bytes}));first_digest=Some(source_digest);
 }
 println!("{}",json!({"export":builder.build().unwrap(),"sources":sources}));
}
"#).unwrap();
    let output=Command::new("cargo").args(["run","--quiet","--manifest-path"]).arg(dir.path().join("Cargo.toml")).env("CARGO_BUILD_JOBS","2").env("CARGO_TARGET_DIR",dir.path().join("target")).output().unwrap();
    assert!(output.status.success(),"{}",String::from_utf8_lossy(&output.stderr));serde_json::from_slice(&output.stdout).unwrap()
}

#[tokio::test]
async fn actual_public_export_resolves_stages_atomically_and_preserves_sources_after_restart(){
    let produced=produced();
    let bytes=serde_json::to_vec(&produced["export"]).unwrap();
    let mut bundle=decode_context_export(&bytes).unwrap();
    let dir=tempfile::tempdir().unwrap();let actor=ActorEngine::open(config(dir.path())).await.unwrap();
    let mut registry=IdentityRegistry::new();registry.bind(bundle.scope.clone(),29,vec![]).unwrap();
    let before=actor.stats().await.unwrap().log_events;
    assert!(import_batch(&actor,&registry,&bundle,1).await.is_err());assert_eq!(actor.stats().await.unwrap().log_events,before);
    bundle.context_sources=serde_json::from_value::<Vec<ContextSourceMaterial>>(produced["sources"].clone()).unwrap();bundle.digest=bundle.computed_digest().unwrap();
    let mut corrupted=bundle.clone();corrupted.context_sources[0].original_bytes.push(9);corrupted.digest=corrupted.computed_digest().unwrap();
    assert!(import_batch(&actor,&registry,&corrupted,1).await.is_err());assert_eq!(actor.stats().await.unwrap().log_events,before);
    let mut escalated=bundle.clone();escalated.context_sources[0].item["role"]=serde_json::json!("system");escalated.digest=escalated.computed_digest().unwrap();
    assert!(import_batch(&actor,&registry,&escalated,1).await.is_err());assert_eq!(actor.stats().await.unwrap().log_events,before);
    let partial=import_batch(&actor,&registry,&bundle,1).await.unwrap();assert!(!partial.complete);assert_eq!(partial.accepted,1);
    assert!(context_history::replay(&actor,&bundle.scope,"imported-session","imported-session").await.unwrap().history.messages().is_empty());
    actor.shutdown().await.unwrap();let actor=ActorEngine::open(config(dir.path())).await.unwrap();
    let receipt=import_batch(&actor,&registry,&bundle,1).await.unwrap();assert!(receipt.complete);
    let state=context_history::replay(&actor,&bundle.scope,"imported-session","imported-session").await.unwrap();
    assert_eq!(state.history.messages().len(),2);assert_eq!(state.history.visible_messages()[0].id,"source-1");
    assert_eq!(state.history.message("source-0").unwrap().recorded_at_ns,1791288000123456789);
    assert_eq!(state.history.message("source-0").unwrap().occurred_at_ns,None);
    assert_eq!(state.history.message("source-0").unwrap().authority,hm_context::Authority::ExternalObserved);
    for (index,material) in bundle.context_sources.iter().enumerate(){let id=format!("source-{index}");assert_eq!(state.history.recover(&bundle.scope,&state.history.source_span(&id).unwrap()).unwrap(),material.original_bytes);}
    let count=actor.stats().await.unwrap().log_events;assert_eq!(import_batch(&actor,&registry,&bundle,10).await.unwrap(),receipt);assert_eq!(actor.stats().await.unwrap().log_events,count);
    let request=hm_serve::context_history::ForkRequest{version:1,scope:bundle.scope.clone(),parent_session_id:"imported-session".into(),parent_conversation:"imported-session".into(),child_session_id:"child".into(),child_conversation:"child".into()};context_history::fork(&actor,&bundle.scope,&request).await.unwrap();
    let mcp=hm_mcp::McpServer::new(actor.clone());let target=state.source_uris[1].rsplit('/').next().unwrap().parse().unwrap();
    assert!(mcp.forget_envelope(hm_mcp::ForgetInput{action:hm_mcp::ForgetAction::Fade,lsn:Some(target),run_id:None,admin_token:None}).await.ok);
    assert!(context_history::replay(&actor,&bundle.scope,"child","child").await.unwrap().history.visible_messages().is_empty());
    actor.shutdown().await.unwrap();let actor=ActorEngine::open(config(dir.path())).await.unwrap();
    assert_eq!(import_batch(&actor,&registry,&bundle,10).await.unwrap(),receipt);
    let state=context_history::replay(&actor,&bundle.scope,"imported-session","imported-session").await.unwrap();assert!(state.history.visible_messages().is_empty());
    let foreign=Scope{owner_id:"foreign".into(),project_id:"project".into(),workspace_id:None};assert!(context_history::recover(&actor,&foreign,"imported-session","imported-session",&state.history.source_span("source-1").unwrap()).await.is_err());
    actor.shutdown().await.unwrap();
}
