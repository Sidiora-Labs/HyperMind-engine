use crate::actor::{ActorEngine, IncomingEvent};
use hm_context::{historian::{Historian, HistorianClaim, HistorianJob, HistorianResult, JobState, SourceChunk}, maintenance::{JobKind, JobStatus, JobLease, JobRequest, MaintenanceScheduler, PublicationFence, SchedulerConfig, SchedulerSnapshot, Usage}, notes::{NotesCommand, NotesEvent, NotesProjection}, types::*};
use hm_core::{ConversationId, Error, ErrorCode, LSN};
use hm_schema::{event::{self, Boundary, CURRENT_SCHEMA_VERSION}, events::{EventEnvelope, EventPayload, ProviderFrame, Retention, Sensitivity}};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use tokio::sync::Mutex;

const PROVIDER: &str = "hypermind/context-jobs/v1";
pub static CONTEXT_MUTATIONS: Mutex<()> = Mutex::const_new(());

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContextJobRequest { pub version: u32, pub scope: Scope, pub request_id: String, pub action: ContextJobAction }

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag="action", rename_all="snake_case", deny_unknown_fields)]
pub enum ContextJobAction {
    Notes { command: NotesCommand },
    ReadNote { id: String, #[serde(with="hm_context::types::timestamp_wire")] now_ns: i64, facts: BTreeMap<String,String> },
    ReadAttribute { key: String },
    SetPolicy { session_id: String, expected_revision: u64, revision: u64 },
    HistorianEnqueue { session_id: String, cursor: Cursor, policy_revision: u64, chunk: SourceChunk, #[serde(default = "default_reservation")] reservation: u64, #[serde(default)] now_ms: u64 },
    HistorianClaim { worker: String, #[serde(default)] now_ms: u64, lease_ms: u64 },
    HistorianHeartbeat { claim: HistorianClaim, #[serde(default)] now_ms: u64, lease_ms: u64 },
    HistorianComplete { claim: HistorianClaim, result: HistorianResult, #[serde(default)] usage: Usage, #[serde(default)] now_ms: u64 },
    HistorianFail { claim: HistorianClaim, #[serde(default)] usage: Usage, #[serde(default)] now_ms: u64, cooldown_ms: u64 },
    HistorianCancel { id: String },
    HistorianExpire { #[serde(default)] now_ms: u64, cooldown_ms: u64 },
    MaintenanceEnqueue { session_id: String, request: JobRequest },
    MaintenanceClaim { #[serde(default)] now_ms: u64 },
    MaintenanceComplete { lease: JobLease, usage: Usage, output_digest: String, #[serde(default)] now_ms: u64 },
    MaintenanceFail { lease: JobLease, usage: Usage, #[serde(default)] now_ms: u64 },
    MaintenanceCancel { id: String, #[serde(default)] now_ms: u64 },
    MaintenanceSettle { id: String, attempt: u64, actual: u64 },
    Inspect,
}

#[derive(Clone, Serialize, Deserialize)]
struct Receipt { digest: String, value: Value }
#[derive(Clone, Serialize, Deserialize)]
struct DurableState {
    version: u32,
    scope: Scope,
    sequence: u64,
    notes: Vec<NotesEvent>,
    historian: Historian,
    maintenance: SchedulerSnapshot,
    maintenance_sessions: BTreeMap<String,String>,
    #[serde(default)]
    historian_maintenance: BTreeMap<String,String>,
    #[serde(default)]
    historian_leases: BTreeMap<String,JobLease>,
    policies: BTreeMap<String,u64>,
    receipts: BTreeMap<String,Receipt>,
}
fn invalid() -> Error { Error::new(ErrorCode::InvalidArgument) }
fn context(error: ContextError) -> Error {
    Error::new(match error { ContextError::ScopeMismatch => ErrorCode::InvalidArgument, ContextError::Capacity => ErrorCode::CapacityExceeded, _ => ErrorCode::InvalidArgument })
}
fn encoded(value: impl Serialize) -> Result<Value,Error> { serde_json::to_value(value).map_err(|_|invalid()) }
fn new_state(scope: &Scope) -> Result<DurableState,Error> {
    Ok(DurableState { version: 1, scope: scope.clone(), sequence: 0, notes: vec![], historian: Historian::new(scope.clone()).map_err(context)?, maintenance: MaintenanceScheduler::new(scope.clone(), SchedulerConfig { max_concurrency: 2, budget: 1_000_000, lease_ms: 60_000, backoff_ms: 1000, max_attempts: 3 }).map_err(context)?.snapshot(), maintenance_sessions: BTreeMap::new(), historian_maintenance: BTreeMap::new(), historian_leases: BTreeMap::new(), policies: BTreeMap::new(), receipts: BTreeMap::new() })
}
async fn load(actor: &ActorEngine, scope: &Scope) -> Result<DurableState,Error> {
    scope.validate().map_err(context)?;
    let mut state = new_state(scope)?;
    for frame in actor.frames_since(LSN::new(0),None,usize::MAX).await? {
        if frame.header.kind != hm_ledger::frame::EventKind::ProviderFrame { continue; }
        let verified = event::verify_event(&frame.sealed_payload,event::EventKind::ProviderFrame,Boundary::Disk)?;
        let EventPayload::ProviderFrame(provider) = verified.envelope.payload else { continue; };
        if provider.provider != PROVIDER { continue; }
        let next: DurableState = serde_json::from_slice(&provider.api_content).map_err(|_|invalid())?;
        if next.scope != *scope || next.version != 1 || next.sequence != state.sequence.checked_add(1).ok_or_else(invalid)? { return Err(invalid()); }
        NotesProjection::restore(scope.clone(),next.notes.clone()).map_err(context)?;
        MaintenanceScheduler::from_snapshot(next.maintenance.clone()).map_err(context)?;
        state = next;
    }
    Ok(state)
}
async fn history(actor: &ActorEngine,scope: &Scope,session: &str) -> Result<hm_context::history::SourceHistory,Error> {
    let mut conversation = None;
    for frame in actor.frames_since(LSN::new(0), None, usize::MAX).await? {
        if frame.header.kind != hm_ledger::frame::EventKind::ProviderFrame { continue; }
        let verified = event::verify_event(&frame.sealed_payload, event::EventKind::ProviderFrame, Boundary::Disk)?;
        let EventPayload::ProviderFrame(provider) = verified.envelope.payload else { continue; };
        if provider.provider != "hypermind/context-session/v1" { continue; }
        let binding: Value = serde_json::from_slice(&provider.api_content).map_err(|_| invalid())?;
        if binding["request"]["session_id"] != session { continue; }
        let bound_scope: Scope = serde_json::from_value(binding["request"]["scope"].clone()).map_err(|_|invalid())?;
        if &bound_scope != scope { return Err(invalid()); }
        conversation = Some(binding["conversation"].as_str().ok_or_else(invalid)?.to_string());
    }
    let conversation = conversation.ok_or_else(invalid)?;
    Ok(crate::context_history::replay(actor,scope,session,&conversation).await.map_err(|_|invalid())?.history)
}
fn validate_chunk(history: &hm_context::history::SourceHistory, chunk: &SourceChunk) -> Result<(),Error> {
    chunk.validate().map_err(context)?;
    let visible = history.visible_messages();
    for (source,span) in chunk.sources.iter().zip(&chunk.spans) {
        if !visible.iter().any(|current| *current == source) || history.source_span(&source.id).map_err(context)? != *span { return Err(invalid()); }
        if source.authority == Authority::DerivedInference { return Err(invalid()); }
    }
    Ok(())
}
fn policy(state: &DurableState,session: &str) -> u64 { state.policies.get(session).copied().unwrap_or(1) }

pub async fn execute(actor: &ActorEngine, trusted_scope: &Scope, trusted_principal: &str, request: ContextJobRequest) -> Result<Value,Error> {
    if request.version != 1 || request.scope != *trusted_scope { return Err(invalid()); }
    validate_id(&request.request_id).map_err(context)?;
    validate_id(trusted_principal).map_err(context)?;
    let _guard = CONTEXT_MUTATIONS.lock().await;
    let expected_tail = actor.stats().await?.applied.last_lsn;
    let mut state = load(actor,trusted_scope).await?;
    let now_ms = runtime_now_ms()?;
    let owner = trusted_principal == trusted_scope.owner_id;
    if !owner && !matches!(&request.action,ContextJobAction::Notes{..}|ContextJobAction::ReadNote{..}) { return Err(invalid()); }
    let key = digest_bytes(&serde_json::to_vec(&(trusted_principal,&request.request_id)).map_err(|_|invalid())?);
    let digest = digest_bytes(&serde_json::to_vec(&request).map_err(|_|invalid())?);
    if let Some(receipt) = state.receipts.get(&key) {
        if receipt.digest != digest { return Err(invalid()); }
        return Ok(receipt.value.clone());
    }
    let mut scheduler = MaintenanceScheduler::from_snapshot(state.maintenance.clone()).map_err(context)?;
    let mut notes = NotesProjection::restore(trusted_scope.clone(),state.notes.clone()).map_err(context)?;
    scheduler.expire(now_ms).map_err(context)?;
    let expired_historian = state.historian.expire(now_ms,scheduler.snapshot().config.backoff_ms).map_err(context)?;
    state.historian_leases.retain(|_,lease|scheduler.snapshot().jobs.get(&lease.job_id).is_some_and(|job|job.status == JobStatus::Running(lease.clone())));
    let reply = match request.action {
        ContextJobAction::Notes{command} => {
            let event = notes.plan(trusted_principal,command).map_err(context)?;
            notes.replay(event.clone()).map_err(context)?;
            state.notes.push(event.clone()); encoded(event)?
        }
        ContextJobAction::ReadNote{id,now_ns,facts} => return encoded(notes.read(trusted_principal,&id,now_ns,&facts).map_err(context)?),
        ContextJobAction::ReadAttribute{key} => return encoded(notes.attribute(trusted_principal,&key).map_err(context)?),
        ContextJobAction::SetPolicy{session_id,expected_revision,revision} => {
            validate_id(&session_id).map_err(context)?;
            if expected_revision != policy(&state,&session_id) || revision != expected_revision.checked_add(1).ok_or_else(invalid)? { return Err(invalid()); }
            state.policies.insert(session_id,revision); json!({"policy_revision":revision})
        }
        ContextJobAction::HistorianEnqueue{session_id,cursor,policy_revision,chunk,reservation,now_ms:_} => {
            let current = history(actor,trusted_scope,&session_id).await?;
            validate_chunk(&current,&chunk)?;
            if cursor != current.cursor() || policy_revision != policy(&state,&session_id) { return Err(invalid()); }
            let maintenance_id = scheduler.enqueue(JobRequest { kind: JobKind::Historian, sources: chunk.sources.clone(), cursor, source_revision: cursor.sequence, policy_revision, reservation }).map_err(context)?;
            let id = state.historian.enqueue(&session_id,cursor,policy_revision,chunk,now_ms).map_err(context)?;
            if state.maintenance_sessions.get(&maintenance_id).is_some_and(|session| session != &session_id) { return Err(invalid()); }
            state.historian_maintenance.insert(id.clone(),maintenance_id.clone());
            state.maintenance_sessions.insert(maintenance_id,session_id);
            encoded(id)?
        }
        ContextJobAction::HistorianClaim{worker,now_ms:_,lease_ms} => {
            let lease_ms = bounded_lease(lease_ms)?;
            let candidates: Vec<_> = state.historian.jobs().filter(|job|matches!(job.state,JobState::Pending{ready_at_ms} if ready_at_ms <= now_ms)).cloned().collect();
            let mut reserved = None;
            for job in candidates {
                let maintenance_id = if let Some(id) = state.historian_maintenance.get(&job.id) { id.clone() } else {
                    let id = scheduler.enqueue(JobRequest { kind: JobKind::Historian,sources:job.chunk.sources.clone(),cursor:job.cursor,source_revision:job.cursor.sequence,policy_revision:job.policy_revision,reservation:default_reservation() }).map_err(context)?;
                    state.historian_maintenance.insert(job.id.clone(),id.clone());
                    state.maintenance_sessions.insert(id.clone(),job.session_id.clone());
                    id
                };
                if let Some(lease) = scheduler.claim_job(&maintenance_id,now_ms,lease_ms).map_err(context)? { reserved = Some((job.id,lease)); break; }
            }
            if let Some((id,lease)) = reserved {
                let claim = state.historian.claim_job(&id,&worker,now_ms,lease_ms).map_err(context)?.ok_or_else(invalid)?;
                state.historian_leases.insert(id,lease);
                encoded(Some(claim))?
            } else { Value::Null }
        },
        ContextJobAction::HistorianHeartbeat{claim,now_ms:_,lease_ms} => {
            let lease_ms = bounded_lease(lease_ms)?;
            let lease = historian_lease(&state,&claim)?;
            let renewed = scheduler.heartbeat(&lease,now_ms,lease_ms).map_err(context)?;
            state.historian.heartbeat(&claim,now_ms,lease_ms).map_err(context)?;
            state.historian_leases.insert(claim.job.id,renewed);
            json!({"heartbeat":true})
        }
        ContextJobAction::HistorianComplete{claim,result,usage,now_ms:_} => {
            let job = state.historian.jobs().find(|job|job.id == claim.job.id).cloned().ok_or_else(invalid)?;
            let current = history(actor,trusted_scope,&job.session_id).await?;
            validate_chunk(&current,&job.chunk)?;
            if current.cursor() != job.cursor { return Err(invalid()); }
            let revision = policy(&state,&job.session_id);
            let now_ms = runtime_now_ms()?;
            let lease = historian_lease(&state,&claim)?;
            let output_digest = digest_bytes(&serde_json::to_vec(&result).map_err(|_|invalid())?);
            let current_fence = PublicationFence { input_digest: lease.fence.input_digest.clone(), source_revision: current.cursor().sequence, policy_revision: revision };
            scheduler.complete(&lease,&current_fence,usage,&output_digest,now_ms).map_err(context)?;
            state.historian.complete(&claim,now_ms,&job.chunk,revision,result).map_err(context)?;
            state.historian_leases.remove(&job.id);
            json!({"job_id":job.id,"published":true,"context_applied":false,"authority":"derived_inference"})
        }
        ContextJobAction::HistorianFail{claim,usage,now_ms:_,cooldown_ms} => {
            let lease = historian_lease(&state,&claim)?;
            scheduler.fail(&lease,usage,now_ms).map_err(context)?;
            state.historian.fail(&claim,now_ms,cooldown_ms.min(60_000)).map_err(context)?;
            state.historian_leases.remove(&claim.job.id);
            json!({"failed":true})
        }
        ContextJobAction::HistorianCancel{id} => {
            if let Some(maintenance_id) = state.historian_maintenance.get(&id) {
                if scheduler.snapshot().jobs.get(maintenance_id).is_some_and(|job|job.status != JobStatus::Complete) { scheduler.cancel(maintenance_id,now_ms).map_err(context)?; }
            }
            state.historian.cancel(&id).map_err(context)?;
            state.historian_leases.remove(&id);
            json!({"cancelled":true})
        }
        ContextJobAction::HistorianExpire{now_ms:_,cooldown_ms:_} => encoded(expired_historian)?,
        ContextJobAction::MaintenanceEnqueue{session_id,request} => {
            if request.kind == JobKind::Historian { return Err(invalid()); }
            let current = history(actor,trusted_scope,&session_id).await?;
            if request.cursor != current.cursor() || request.source_revision != current.cursor().sequence || request.policy_revision != policy(&state,&session_id) { return Err(invalid()); }
            let visible = current.visible_messages();
            if request.sources.iter().any(|source|!visible.iter().any(|message|*message==source)) { return Err(invalid()); }
            let id = scheduler.enqueue(request).map_err(context)?;
            if state.maintenance_sessions.get(&id).is_some_and(|existing|existing != &session_id) { return Err(invalid()); }
            state.maintenance_sessions.insert(id.clone(),session_id); encoded(id)?
        }
        ContextJobAction::MaintenanceClaim{now_ms:_} => encoded(scheduler.claim_non_historian(now_ms).map_err(context)?)?,
        ContextJobAction::MaintenanceComplete{lease,usage,output_digest,now_ms:_} => {
            let session = state.maintenance_sessions.get(&lease.job_id).ok_or_else(invalid)?;
            let current = history(actor,trusted_scope,session).await?;
            let snapshot = scheduler.snapshot();
            let job = snapshot.jobs.get(&lease.job_id).ok_or_else(invalid)?;
            if job.request.kind == JobKind::Historian { return Err(invalid()); }
            let visible = current.visible_messages();
            if job.request.sources.iter().any(|source|!visible.iter().any(|message|*message==source)) || current.cursor() != job.request.cursor { return Err(invalid()); }
            let identities: BTreeMap<_,_> = job.request.sources.iter().map(|source|(source.id.clone(),source.source_digest.clone())).collect();
            let fence = PublicationFence { input_digest: digest_bytes(&serde_json::to_vec(&identities).map_err(|_|invalid())?), source_revision: current.cursor().sequence, policy_revision: policy(&state,session) };
            encoded(scheduler.complete(&lease,&fence,usage,&output_digest,runtime_now_ms()?).map_err(context)?)?
        }
        ContextJobAction::MaintenanceFail{lease,usage,now_ms:_} => {
            if scheduler.snapshot().jobs.get(&lease.job_id).is_some_and(|job|job.request.kind == JobKind::Historian) { return Err(invalid()); }
            scheduler.fail(&lease,usage,now_ms).map_err(context)?; json!({"failed":true}) }
        ContextJobAction::MaintenanceCancel{id,now_ms:_} => {
            if scheduler.snapshot().jobs.get(&id).is_some_and(|job|job.request.kind == JobKind::Historian) { return Err(invalid()); }
            scheduler.cancel(&id,now_ms).map_err(context)?; json!({"cancelled":true}) }
        ContextJobAction::MaintenanceSettle{id,attempt,actual} => { scheduler.settle_unknown(&id,attempt,actual).map_err(context)?; json!({"settled":true}) }
        ContextJobAction::Inspect => return inspect_state(actor,trusted_scope,trusted_principal).await,
    };
    state.maintenance = scheduler.snapshot();
    state.sequence = state.sequence.checked_add(1).ok_or_else(invalid)?;
    let reply = json!({"version":1,"sequence":state.sequence,"request_id":request.request_id,"result":reply});
    state.receipts.insert(key,Receipt{digest,value:reply.clone()});
    actor.append_if_tail(expected_tail,vec![IncomingEvent { kind: hm_ledger::frame::EventKind::ProviderFrame, conversation: ConversationId::derive(PROVIDER), payload: event::encode_event_envelope(&EventEnvelope { schema_version: CURRENT_SCHEMA_VERSION, payload: EventPayload::ProviderFrame(Box::new(ProviderFrame { provider: PROVIDER.into(), api_content: serde_json::to_vec(&state).map_err(|_|invalid())? })), connection_id: None, client_seq: 0, client_event_index: 0, client_event_count: 1, origin_actor: 0, run_id: None, model_provenance: None, authority: hm_schema::events::Authority::RuntimeFact, retention: Retention::Durable, sensitivity: Sensitivity::Personal, event_time_ns: 0 }) }]).await?;
    Ok(reply)
}

pub async fn published_summaries(actor:&ActorEngine,scope:&Scope,session_id:&str)->Result<Vec<HistorianJob>,Error> {
    let state = load(actor,scope).await?;
    Ok(state.historian.jobs().filter(|job|job.session_id==session_id && matches!(job.state,JobState::Complete)).cloned().collect())
}

pub async fn inspect_state(actor:&ActorEngine,scope:&Scope,trusted_principal:&str)->Result<Value,Error> {
    if trusted_principal != scope.owner_id { return Err(invalid()); }
    let state = load(actor,scope).await?;
    let notes = NotesProjection::restore(scope.clone(),state.notes.clone()).map_err(context)?;
    let permission_revision = digest_bytes(&serde_json::to_vec(&state.notes).map_err(|_|invalid())?);
    Ok(json!({"version":1,"scope":scope,"sequence":state.sequence,"notes":state.notes,"historian":state.historian.jobs().collect::<Vec<_>>(),"maintenance":state.maintenance,"proposals":notes.proposals(trusted_principal).map_err(context)?,"policies":state.policies,"permission_revision":permission_revision,"historian_maintenance":state.historian_maintenance,"historian_leases":state.historian_leases,"watermark_semantics":"completed prefix of registered jobs; gaps enumerate unfinished registered cursors"}))
}

fn default_reservation() -> u64 { 4096 }
fn runtime_now_ms() -> Result<u64,Error> {
    let elapsed = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_err(|_|invalid())?;
    u64::try_from(elapsed.as_millis()).map_err(|_|invalid())
}
fn bounded_lease(lease_ms:u64)->Result<u64,Error> {
    if lease_ms == 0 || lease_ms > 60_000 { return Err(invalid()); }
    Ok(lease_ms)
}
fn historian_lease(state:&DurableState,claim:&HistorianClaim)->Result<JobLease,Error> {
    let job = state.historian.jobs().find(|job|job.id == claim.job.id).ok_or_else(invalid)?;
    if job.attempt != claim.attempt { return Err(invalid()); }
    state.historian_leases.get(&claim.job.id).cloned().ok_or_else(invalid)
}
