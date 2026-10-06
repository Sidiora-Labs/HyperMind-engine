use crate::transport::AuthenticatedIdentity;
use crate::Scope;
use hm_context::types::validate_id;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ModuleManifest {
    pub module_id: String,
    pub protocol_version: u32,
    pub capabilities: BTreeMap<String, u32>,
    pub max_calls: usize,
    pub max_bytes: usize,
    pub queue_calls: usize,
    pub queue_bytes: usize,
}
impl ModuleManifest {
    pub fn validate(&self) -> Result<(), RouteError> {
        validate_id(&self.module_id).map_err(|_| RouteError::Invalid)?;
        if self.protocol_version != 1 || self.capabilities.is_empty() || self.capabilities.len() > 128
            || self.max_calls == 0 || self.max_bytes == 0 || self.queue_calls == 0 || self.queue_bytes == 0
            || self.max_calls > 65536 || self.queue_calls > 65536 || self.max_bytes > 64*1024*1024 || self.queue_bytes > 64*1024*1024 {
            return Err(RouteError::Invalid);
        }
        for (name, version) in &self.capabilities {
            validate_id(name).map_err(|_| RouteError::Invalid)?;
            if *version == 0 { return Err(RouteError::Invalid); }
        }
        Ok(())
    }
    pub fn negotiate(&self, required: &BTreeMap<String, u32>) -> Result<BTreeMap<String,u32>,RouteError> {
        self.validate()?;
        if required.iter().any(|(name, version)| self.capabilities.get(name) != Some(version)) { return Err(RouteError::Capability); }
        Ok(required.clone())
    }
}
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct Binding {
    pub scope: Scope,
    pub module_id: String,
    pub launch_id: String,
    pub generation: u64,
    pub epoch: u64,
    pub session_id: [u8;32],
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum EffectState { NotDispatched, Dispatched, Unknown, Completed }
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum RouteError {
    #[error("invalid manifest or call")] Invalid,
    #[error("authentication or launch mismatch")] Authentication,
    #[error("unsupported capability")] Capability,
    #[error("stale channel binding")] Stale,
    #[error("module draining")] Draining,
    #[error("bounded queue full")] Capacity,
    #[error("call cancelled")] Cancelled,
    #[error("deadline expired")] Deadline,
    #[error("replacement has pending effects")] ReplacementBusy,
    #[error("call identity conflict")] Conflict,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Refusal { pub error: RouteError, pub effect: EffectState }
impl From<RouteError> for Refusal { fn from(error: RouteError)->Self { Self{error,effect:EffectState::NotDispatched} } }
#[derive(Clone, Debug)]
pub struct RouteCall {
    pub id: String,
    pub caller: String,
    pub capability: String,
    pub capability_version: u32,
    pub deadline_unix_ms: Option<u64>,
    pub payload: Vec<u8>,
}
#[derive(Clone, Debug)]
pub struct Dispatch { pub binding: Binding, pub call: RouteCall }
#[derive(Clone, Debug)]
pub struct CallOutcome { pub id: String, pub effect: EffectState, pub error: Option<RouteError> }
struct Module {
    binding: Binding,
    manifest: ModuleManifest,
    draining: bool,
    queues: BTreeMap<String, VecDeque<RouteCall>>,
    turns: VecDeque<String>,
    queued_calls: usize,
    queued_bytes: usize,
    active: BTreeMap<String, RouteCall>,
    active_bytes: usize,
    seen: BTreeSet<String>,
}
#[derive(Default)]
pub struct Router {
    modules: BTreeMap<(Scope,String),Module>,
    launches: BTreeMap<(Scope,String), (String,u64,[u8;32])>,
    epoch: u64,
}
impl Router {
    pub fn new() -> Self { Self::default() }
    pub fn authorize_launch(&mut self, identity: &AuthenticatedIdentity, module_id: &str, launch_id: &str, generation: u64) -> Result<(),RouteError> {
        validate_id(module_id).map_err(|_|RouteError::Invalid)?;
        validate_id(launch_id).map_err(|_|RouteError::Invalid)?;
        if generation == 0 { return Err(RouteError::Invalid); }
        let key = (identity.scope().clone(),module_id.to_string());
        if self.launches.get(&key).is_some_and(|(_,old,_)| generation <= *old) { return Err(RouteError::Stale); }
        if !self.launches.contains_key(&key) && self.launches.len() >= 4096 { return Err(RouteError::Capacity); }
        self.launches.insert(key,(launch_id.to_string(),generation,*identity.peer_key()));
        Ok(())
    }
    pub fn register(&mut self, identity: &AuthenticatedIdentity, manifest: ModuleManifest, launch_id: &str, generation: u64) -> Result<Binding,Refusal> {
        manifest.validate()?;
        let key=(identity.scope().clone(),manifest.module_id.clone());
        if self.launches.get(&key) != Some(&(launch_id.to_string(),generation,*identity.peer_key())) { return Err(RouteError::Authentication.into()); }
        if let Some(old)=self.modules.get(&key) {
            if generation <= old.binding.generation { return Err(RouteError::Stale.into()); }
            if !old.draining || !old.active.is_empty() || old.queued_calls != 0 { return Err(RouteError::ReplacementBusy.into()); }
        }
        self.epoch=self.epoch.checked_add(1).ok_or(RouteError::Capacity)?;
        let binding=Binding{scope:key.0.clone(),module_id:key.1.clone(),launch_id:launch_id.into(),generation,epoch:self.epoch,session_id:*identity.session_id()};
        self.modules.insert(key,Module{binding:binding.clone(),manifest,draining:false,queues:BTreeMap::new(),turns:VecDeque::new(),queued_calls:0,queued_bytes:0,active:BTreeMap::new(),active_bytes:0,seen:BTreeSet::new()});
        Ok(binding)
    }
    fn module(&mut self,binding:&Binding)->Result<&mut Module,RouteError> {
        self.modules.get_mut(&(binding.scope.clone(),binding.module_id.clone())).filter(|m|m.binding==*binding).ok_or(RouteError::Stale)
    }
    pub fn enqueue(&mut self, identity:&AuthenticatedIdentity, binding:&Binding, call:RouteCall, now_ms:u64)->Result<(),Refusal> {
        if identity.scope()!=&binding.scope { return Err(RouteError::Authentication.into()); }
        let module=self.module(binding)?;
        for id in [&call.id,&call.caller,&call.capability] { validate_id(id).map_err(|_|RouteError::Invalid)?; }
        if module.draining { return Err(RouteError::Draining.into()); }
        if call.deadline_unix_ms.is_some_and(|d|d<=now_ms) { return Err(RouteError::Deadline.into()); }
        if module.manifest.capabilities.get(&call.capability)!=Some(&call.capability_version) { return Err(RouteError::Capability.into()); }
        if module.seen.contains(&call.id) { return Err(RouteError::Conflict.into()); }
        if call.payload.len()>module.manifest.max_bytes || module.queued_calls>=module.manifest.queue_calls || call.payload.len()>module.manifest.queue_bytes.saturating_sub(module.queued_bytes) || module.seen.len()>=65536 { return Err(RouteError::Capacity.into()); }
        // Caller scheduling identities are pinned to the authenticated peer.
        let caller=identity.peer_key().iter().map(|b|format!("{b:02x}")).collect::<String>();
        let mut call=call; call.caller=caller.clone();
        if !module.queues.contains_key(&caller) { module.turns.push_back(caller.clone()); }
        module.seen.insert(call.id.clone()); module.queued_calls+=1; module.queued_bytes+=call.payload.len();
        module.queues.entry(caller).or_default().push_back(call);
        Ok(())
    }
    pub fn dispatch(&mut self,binding:&Binding,now_ms:u64)->Result<(Option<Dispatch>,Vec<CallOutcome>),Refusal> {
        let module=self.module(binding)?;
        let mut outcomes=Vec::new();
        for _ in 0..module.turns.len() {
            let Some(caller)=module.turns.pop_front() else { break; };
            let queue=module.queues.get_mut(&caller).ok_or(RouteError::Invalid)?;
            while queue.front().is_some_and(|c|c.deadline_unix_ms.is_some_and(|d|d<=now_ms)) {
                let call=queue.pop_front().ok_or(RouteError::Invalid)?;
                module.queued_calls-=1; module.queued_bytes-=call.payload.len();
                outcomes.push(CallOutcome{id:call.id,effect:EffectState::NotDispatched,error:Some(RouteError::Deadline)});
            }
            let fits=queue.front().is_some_and(|c|module.active.len()<module.manifest.max_calls && c.payload.len()<=module.manifest.max_bytes.saturating_sub(module.active_bytes));
            let dispatched=if fits { queue.pop_front() } else { None };
            if queue.is_empty() { module.queues.remove(&caller); } else { module.turns.push_back(caller); }
            if let Some(call)=dispatched {
                module.queued_calls-=1; module.queued_bytes-=call.payload.len(); module.active_bytes+=call.payload.len(); module.active.insert(call.id.clone(),call.clone());
                return Ok((Some(Dispatch{binding:binding.clone(),call}),outcomes));
            }
        }
        Ok((None,outcomes))
    }
    pub fn complete(&mut self,binding:&Binding,id:&str)->Result<CallOutcome,Refusal> {
        let module=self.module(binding)?;
        let call=module.active.remove(id).ok_or(RouteError::Conflict)?; module.active_bytes-=call.payload.len();
        Ok(CallOutcome{id:id.into(),effect:EffectState::Completed,error:None})
    }
    pub fn cancel(&mut self,binding:&Binding,id:&str)->Result<CallOutcome,Refusal> {
        let module=self.module(binding)?;
        if module.active.contains_key(id) { return Ok(CallOutcome{id:id.into(),effect:EffectState::Unknown,error:Some(RouteError::Cancelled)}); }
        for queue in module.queues.values_mut() {
            if let Some(index)=queue.iter().position(|c|c.id==id) {
                let call=queue.remove(index).ok_or(RouteError::Conflict)?; module.queued_calls-=1; module.queued_bytes-=call.payload.len();
                return Ok(CallOutcome{id:id.into(),effect:EffectState::NotDispatched,error:Some(RouteError::Cancelled)});
            }
        }
        Err(RouteError::Conflict.into())
    }
    pub fn drain(&mut self,binding:&Binding)->Result<Vec<CallOutcome>,Refusal> {
        let module=self.module(binding)?; module.draining=true;
        let outcomes=module.queues.values_mut().flat_map(|q|q.drain(..)).map(|call|CallOutcome{id:call.id,effect:EffectState::NotDispatched,error:Some(RouteError::Draining)}).collect();
        module.queues.clear(); module.turns.clear(); module.queued_calls=0; module.queued_bytes=0;
        Ok(outcomes)
    }
    pub fn is_drained(&mut self,binding:&Binding)->Result<bool,Refusal> { let module=self.module(binding)?; Ok(module.draining&&module.active.is_empty()&&module.queued_calls==0) }
}
pub type Registry = Router;
