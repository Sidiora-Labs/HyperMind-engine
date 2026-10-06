use std::collections::BTreeMap;
use ed25519_dalek::SigningKey;
use hm_fabric::{Scope, routing::*, transport::*};
use tokio::net::UnixStream;
async fn identity(seed:u8,scope:Scope)->AuthenticatedIdentity {
    let first=SigningKey::from_bytes(&[seed;32]); let second=SigningKey::from_bytes(&[seed+1;32]);
    let (a,b)=UnixStream::pair().unwrap();
    let (a,b)=tokio::join!(UnixTransport::accept(a,Credentials{scope:scope.clone(),peer_key:second.verifying_key(),signing_key:first.clone()},ReplayGuard::default(),Limits::default()),UnixTransport::accept(b,Credentials{scope,peer_key:first.verifying_key(),signing_key:second},ReplayGuard::default(),Limits::default()));
    let a=a.unwrap(); b.unwrap(); a.identity().clone()
}
fn scope()->Scope { Scope{owner_id:"owner".into(),project_id:"project".into(),workspace_id:None} }
fn manifest()->ModuleManifest { ModuleManifest{module_id:"worker".into(),protocol_version:1,capabilities:BTreeMap::from([("run".into(),1)]),max_calls:1,max_bytes:8,queue_calls:4,queue_bytes:16} }
fn call(id:&str,size:usize)->RouteCall { RouteCall{id:id.into(),caller:"caller".into(),capability:"run".into(),capability_version:1,deadline_unix_ms:None,payload:vec![1;size]} }
async fn setup()->(Router,AuthenticatedIdentity,Binding) { let identity=identity(10,scope()).await; let mut router=Router::new(); router.authorize_launch(&identity,"worker","launch-1",1).unwrap(); let binding=router.register(&identity,manifest(),"launch-1",1).unwrap(); (router,identity,binding) }
#[tokio::test]
async fn validation_authentication_and_capability() {
 let (mut router,authenticated,binding)=setup().await;
 let other=identity(20,scope()).await;
 assert_eq!(router.register(&other,manifest(),"launch-1",1).unwrap_err().error,RouteError::Authentication);
 let mut bad=manifest(); bad.capabilities.insert("run".into(),0); assert_eq!(bad.validate(),Err(RouteError::Invalid));
 assert_eq!(manifest().negotiate(&BTreeMap::from([("run".into(),2)])),Err(RouteError::Capability));
 let foreign=identity(30,Scope{project_id:"other".into(),..scope()}).await;
 assert_eq!(router.enqueue(&foreign,&binding,call("a",1),0).unwrap_err().effect,EffectState::NotDispatched);
 router.enqueue(&authenticated,&binding,call("a",1),0).unwrap();
}
#[tokio::test]
async fn credits_fairness_and_bounded_queue() {
 let (mut router,first,binding)=setup().await; let second=identity(20,scope()).await;
 router.enqueue(&first,&binding,call("a",4),0).unwrap(); router.enqueue(&first,&binding,call("b",4),0).unwrap(); router.enqueue(&second,&binding,call("c",4),0).unwrap();
 assert_eq!(router.enqueue(&first,&binding,call("large",9),0).unwrap_err().error,RouteError::Capacity);
 let (dispatch,_)=router.dispatch(&binding,0).unwrap(); assert_eq!(dispatch.unwrap().call.id,"a");
 assert!(router.dispatch(&binding,0).unwrap().0.is_none()); router.complete(&binding,"a").unwrap();
 assert_eq!(router.dispatch(&binding,0).unwrap().0.unwrap().call.id,"c"); router.complete(&binding,"c").unwrap();
 assert_eq!(router.dispatch(&binding,0).unwrap().0.unwrap().call.id,"b");
}
#[tokio::test]
async fn cancellation_deadlines_and_replacement_are_effect_aware() {
 let (mut router,identity,binding)=setup().await;
 let mut timed=call("timed",1); timed.deadline_unix_ms=Some(10); router.enqueue(&identity,&binding,timed,0).unwrap();
 let (none,outcomes)=router.dispatch(&binding,10).unwrap(); assert!(none.is_none()); assert_eq!(outcomes[0].effect,EffectState::NotDispatched);
 router.enqueue(&identity,&binding,call("cancel",1),0).unwrap(); assert_eq!(router.cancel(&binding,"cancel").unwrap().effect,EffectState::NotDispatched);
 router.enqueue(&identity,&binding,call("active",1),0).unwrap(); router.dispatch(&binding,0).unwrap();
 assert_eq!(router.cancel(&binding,"active").unwrap().effect,EffectState::Unknown);
 router.authorize_launch(&identity,"worker","launch-2",2).unwrap(); router.drain(&binding).unwrap();
 assert_eq!(router.register(&identity,manifest(),"launch-2",2).unwrap_err().error,RouteError::ReplacementBusy);
 assert_eq!(router.enqueue(&identity,&binding,call("refused",1),0).unwrap_err().effect,EffectState::NotDispatched);
 router.complete(&binding,"active").unwrap(); assert!(router.is_drained(&binding).unwrap());
 let new=router.register(&identity,manifest(),"launch-2",2).unwrap(); assert!(new.epoch>binding.epoch);
 assert_eq!(router.dispatch(&binding,0).unwrap_err().error,RouteError::Stale);
 let mut forged=new.clone(); forged.session_id=[0;32]; assert_eq!(router.dispatch(&forged,0).unwrap_err().error,RouteError::Stale);
}
