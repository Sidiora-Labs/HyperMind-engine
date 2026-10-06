use crate::actor::{ActorEngine, IncomingEvent};
use hm_context::{cache::*, historian::{HistorianResult, SourceChunk}, history::SourceHistory, types::*};
use hm_core::{ConversationId, Error, ErrorCode, LSN};
use hm_schema::{event::{self, Boundary, CURRENT_SCHEMA_VERSION}, events::{EventEnvelope, EventPayload, ProviderFrame, Retention, Sensitivity}};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, sync::OnceLock};

const PROVIDER: &str = "hypermind/context-projection/v1";
static WRITER: OnceLock<tokio::sync::Mutex<()>> = OnceLock::new();
fn invalid() -> Error { Error::new(ErrorCode::InvalidArgument) }
fn map(_: ContextError) -> Error { invalid() }
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SummaryLevel { #[default] Detailed, Condensed, Brief, Outline }
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectionRequest {
    pub model_id: String,
    pub policy_revision: String,
    pub permission_revision: String,
    pub required_blocks: Vec<ContextBlock>,
    pub required_message_ids: Vec<String>,
    pub tier: SummaryLevel,
    pub defer_reductions: bool,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContextMaterialization {
    pub messages: Vec<SourceMessage>,
    pub blocks: Vec<ContextBlock>,
    pub coverage: Vec<SourceSpan>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProjectionView {
    pub fence: CacheFence,
    pub generation: u64,
    pub bytes: Vec<u8>,
    pub messages: Vec<SourceMessage>,
    pub blocks: Vec<ContextBlock>,
    pub coverage: Vec<SourceSpan>,
    pub pending_reductions: usize,
    pub replayed: bool,
    pub ledger_tail: u64,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Publication { chunk: SourceChunk, result: HistorianResult }
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct State {
    version: u32,
    revision: u64,
    history: Vec<u8>,
    request: ProjectionRequest,
    cache: CacheCheckpoint,
    summaries: Vec<Publication>,
}
pub fn source_revision(history: &SourceHistory) -> Result<String, ContextError> { Ok(digest_bytes(&history.export_canonical()?)) }
pub fn fence_for(history: &SourceHistory, request: &ProjectionRequest) -> Result<CacheFence, ContextError> {
    for id in [&request.model_id, &request.policy_revision, &request.permission_revision] { validate_id(id)?; }
    let fence = CacheFence { scope: history.scope().clone(), session_id: history.session_id().into(), model_id: request.model_id.clone(), policy_revision: digest_bytes(&serde_json::to_vec(&(&request.policy_revision, &request.permission_revision, &request.required_blocks, &request.required_message_ids, request.tier))?), source_revision: source_revision(history)? };
    fence.validate()?;
    Ok(fence)
}
fn validate_publication(history: &SourceHistory, publication: &Publication) -> Result<(), ContextError> {
    publication.result.validate(&publication.chunk)?;
    let visible: BTreeSet<_> = history.visible_messages().into_iter().map(|m| m.id.as_str()).collect();
    for (source, span) in publication.chunk.sources.iter().zip(&publication.chunk.spans) {
        if !visible.contains(source.id.as_str()) || history.message(&source.id)? != source || &history.source_span(&source.id)? != span { return Err(ContextError::Stale); }
        history.recover(history.scope(), span)?;
    }
    let visible = history.visible_messages();
    let start = visible.iter().position(|m| m.id == publication.chunk.sources[0].id).ok_or(ContextError::Stale)?;
    if visible.get(start..start + publication.chunk.sources.len()).is_none_or(|sources| sources.iter().zip(&publication.chunk.sources).any(|(a,b)| *a != b)) { return Err(ContextError::Stale); }
    Ok(())
}
fn validate_state(state: &State) -> Result<SourceHistory, Error> {
    if state.version != CONTRACT_VERSION || state.revision == 0 { return Err(invalid()); }
    let history = SourceHistory::restore_canonical(&state.history).map_err(map)?;
    let cache = ContextCache::restore(state.cache.clone(), &state.cache.fence).map_err(map)?;
    if history.scope() != &state.cache.fence.scope || history.session_id() != state.cache.fence.session_id || source_revision(&history).map_err(map)? != state.cache.fence.source_revision { return Err(invalid()); }
    let mut ids = BTreeSet::new();
    for publication in &state.summaries { validate_publication(&history, publication).map_err(map)?; if !ids.insert(&publication.chunk.digest) { return Err(invalid()); } }
    if state.cache.invalidation.is_none() {
        if fence_for(&history, &state.request).map_err(map)? != state.cache.fence { return Err(invalid()); }
        let materialization: ContextMaterialization = serde_json::from_slice(&cache.replay(&state.cache.fence).map_err(map)?).map_err(|_| invalid())?;
        let visible = history.visible_messages();
        for message in &materialization.messages { if !visible.contains(&message) { return Err(invalid()); } }
        let expected_coverage = visible.iter().map(|message| history.source_span(&message.id)).collect::<Result<Vec<_>, _>>().map_err(map)?;
        if materialization.coverage != expected_coverage { return Err(invalid()); }
    }
    Ok(history)
}
async fn load(actor: &ActorEngine, scope: &Scope, session: &str) -> Result<Option<State>, Error> {
    scope.validate().map_err(map)?; validate_id(session).map_err(map)?;
    let mut latest: Option<State> = None;
    for frame in actor.frames_since(LSN::new(0), None, usize::MAX).await? {
        if frame.header.kind != hm_ledger::frame::EventKind::ProviderFrame { continue; }
        let verified = event::verify_event(&frame.sealed_payload, event::EventKind::ProviderFrame, Boundary::Disk)?;
        if let EventPayload::ProviderFrame(provider) = verified.envelope.payload {
            if provider.provider != PROVIDER { continue; }
            let state: State = serde_json::from_slice(&provider.api_content).map_err(|_| invalid())?;
            if state.cache.fence.scope != *scope || state.cache.fence.session_id != session { continue; }
            validate_state(&state)?;
            if latest.as_ref().is_some_and(|old| Some(state.revision) != old.revision.checked_add(1)) || latest.is_none() && state.revision != 1 { return Err(invalid()); }
            latest = Some(state);
        }
    }
    Ok(latest)
}
async fn persist(actor: &ActorEngine, state: &mut State, expected_tail: &mut LSN) -> Result<(), Error> {
    state.revision = state.revision.checked_add(1).ok_or_else(invalid)?;
    validate_state(state)?;
    let outcome = actor.append_if_tail(*expected_tail, vec![IncomingEvent {
        kind: hm_ledger::frame::EventKind::ProviderFrame,
        conversation: ConversationId::derive(&format!("context-projection:{}:{}", state.cache.fence.scope.digest().map_err(map)?, state.cache.fence.session_id)),
        payload: event::encode_event_envelope(&EventEnvelope { schema_version: CURRENT_SCHEMA_VERSION, payload: EventPayload::ProviderFrame(Box::new(ProviderFrame { provider: PROVIDER.into(), api_content: serde_json::to_vec(state).map_err(|_| invalid())? })), connection_id: None, client_seq: 0, client_event_index: 0, client_event_count: 1, origin_actor: 0, run_id: None, model_provenance: None, authority: hm_schema::events::Authority::RuntimeFact, retention: Retention::Durable, sensitivity: Sensitivity::Personal, event_time_ns: 0 }),
    }]).await?;
    *expected_tail = outcome.last_lsn;
    Ok(())
}
fn view(state: &State, replayed: bool, ledger_tail: LSN) -> Result<ProjectionView, Error> {
    let cache = ContextCache::restore(state.cache.clone(), &state.cache.fence).map_err(map)?;
    let bytes = cache.replay(&state.cache.fence).map_err(map)?;
    let materialization: ContextMaterialization = serde_json::from_slice(&bytes).map_err(|_| invalid())?;
    Ok(ProjectionView { fence: state.cache.fence.clone(), generation: cache.generation(), bytes, messages: materialization.messages, blocks: materialization.blocks, coverage: materialization.coverage, pending_reductions: cache.pending().len(), replayed, ledger_tail: ledger_tail.get() })
}
fn materialize(history: &SourceHistory, request: &ProjectionRequest, publications: &[Publication]) -> Result<ContextMaterialization, Error> {
    let tokenizer = hm_compose::tokens::TokenCounter::for_model(&request.model_id, None, hm_compose::tokens::FallbackWeights::default())?;
    if matches!(tokenizer, hm_compose::tokens::TokenCounter::Fallback { .. }) { return Err(Error::new(ErrorCode::OperationUnavailable)); }
    let messages = history.visible_messages();
    let mut protected: BTreeSet<String> = request.required_message_ids.iter().cloned().collect();
    if let Some(last) = messages.last() { protected.insert(last.id.clone()); }
    for id in &protected { if !messages.iter().any(|m| &m.id == id) { return Err(invalid()); } }
    let mut covered = BTreeSet::new();
    let mut coverage = Vec::new();
    let mut blocks = request.required_blocks.clone();
    let mut block_ids = BTreeSet::new();
    for block in &mut blocks { validate_id(&block.id).map_err(map)?; if !block_ids.insert(block.id.clone()) { return Err(invalid()); } block.required = true; block.tokens = tokenizer.count(block.text.as_bytes())? as u64; }
    let mut ordered: Vec<_> = publications.iter().collect();
    ordered.sort_by_key(|p| p.chunk.sources[0].ordinal);
    for publication in ordered {
        validate_publication(history, publication).map_err(map)?;
        if publication.chunk.sources.iter().any(|m| protected.contains(&m.id) || covered.contains(&m.id) || m.parts.iter().any(|part| !matches!(part, MessagePart::Text { .. }))) { continue; }
        let tier = &publication.result.tiers[request.tier as usize];
        let id = format!("summary:{}", publication.chunk.digest);
        if !block_ids.insert(id.clone()) { return Err(invalid()); }
        blocks.push(ContextBlock { id, text: tier.text.clone(), authority: Authority::DerivedInference, provenance: tier.coverage.clone(), tokens: tokenizer.count(tier.text.as_bytes())? as u64, required: false });
        coverage.extend(tier.coverage.clone());
        covered.extend(publication.chunk.sources.iter().map(|m| m.id.clone()));
    }
    let messages: Vec<_> = messages.into_iter().filter(|m| !covered.contains(&m.id)).cloned().collect();
    for message in &messages { coverage.push(history.source_span(&message.id).map_err(map)?); }
    coverage.sort_by_key(|span| history.message(&span.source_id).map(|m| m.ordinal).unwrap_or(u64::MAX));
    Ok(ContextMaterialization { messages, blocks, coverage })
}
fn region(history: &SourceHistory, request: &ProjectionRequest, publications: &[Publication]) -> Result<CacheRegion, Error> {
    let materialization = materialize(history, request, publications)?;
    let sources = history.visible_messages().into_iter().map(|m| CacheSource { id: m.id.clone(), digest: m.source_digest.clone(), ordinal: m.ordinal, required: request.required_message_ids.contains(&m.id) }).collect();
    Ok(CacheRegion { bytes: serde_json::to_vec(&materialization).map_err(|_| invalid())?, sources })
}
fn active_region(state: &State) -> RegionKind {
    if !state.cache.baseline.bytes.is_empty() { RegionKind::Baseline } else if !state.cache.delta.bytes.is_empty() { RegionKind::Delta } else { RegionKind::LiveTail }
}
fn queue_materialization(state: &mut State, history: &SourceHistory) -> Result<(), Error> {
    let mut cache = ContextCache::restore(state.cache.clone(), &state.cache.fence).map_err(map)?;
    let target = active_region(state);
    let original = match target { RegionKind::Baseline => &state.cache.baseline, RegionKind::Delta => &state.cache.delta, RegionKind::LiveTail => &state.cache.live_tail };
    let replacement = region(history, &state.request, &state.summaries)?;
    let mut checkpoint = cache.checkpoint().map_err(map)?;
    checkpoint.pending.clear(); checkpoint.digest.clear(); checkpoint.digest = digest_bytes(&serde_json::to_vec(&checkpoint).map_err(|_| invalid())?);
    cache = ContextCache::restore(checkpoint, &state.cache.fence).map_err(map)?;
    cache.queue_change(cache.generation(), PendingChange { id: digest_bytes(&replacement.bytes), region: target, source_identity: original.source_identity().map_err(map)?, replacement }).map_err(map)?;
    state.cache = cache.checkpoint().map_err(map)?;
    Ok(())
}
pub async fn validate_tail(actor: &ActorEngine, expected_tail: LSN) -> Result<(), Error> {
    let actual = actor.stats().await?.applied.last_lsn;
    if actual != expected_tail { return Err(Error::new(ErrorCode::SequenceViolation).at_lsn(actual)); }
    Ok(())
}
pub async fn current(actor: &ActorEngine, scope: &Scope, session: &str) -> Result<Option<ProjectionView>, Error> {
    let tail = actor.stats().await?.applied.last_lsn;
    let state = load(actor, scope, session).await?;
    validate_tail(actor, tail).await?;
    state.as_ref().map(|state| view(state, true, tail)).transpose()
}
pub async fn assemble(actor: &ActorEngine, history: &SourceHistory, request: ProjectionRequest, mut expected_tail: LSN) -> Result<ProjectionView, Error> {
    let _guard = WRITER.get_or_init(|| tokio::sync::Mutex::new(())).lock().await;
    validate_tail(actor, expected_tail).await?;
    let fence = fence_for(history, &request).map_err(map)?;
    let old = load(actor, history.scope(), history.session_id()).await?;
    if let Some(mut state) = old {
        let mut cache = ContextCache::restore(state.cache.clone(), &state.cache.fence).map_err(map)?;
        if state.cache.invalidation.is_some() && state.cache.fence == fence { return Err(invalid()); }
        if state.cache.fence != fence || state.cache.invalidation.is_some() {
            if state.cache.invalidation.is_none() {
                cache.invalidate(SafetyInvalidation::AccessRevoked).map_err(map)?;
                state.cache = cache.checkpoint().map_err(map)?;
                persist(actor, &mut state, &mut expected_tail).await?;
            }
            let permission_changed = state.request.permission_revision != request.permission_revision || state.request.policy_revision != request.policy_revision;
            if permission_changed { state.summaries.clear(); } else { state.summaries.retain(|publication| validate_publication(history, publication).is_ok()); }
            let replacement = region(history, &request, &state.summaries)?;
            cache.reconcile(cache.generation(), &fence, BoundaryChange::Replace { baseline: CacheRegion::default(), delta: CacheRegion::default(), live_tail: replacement }).map_err(map)?;
            state.history = history.export_canonical().map_err(map)?; state.request = request; state.cache = cache.checkpoint().map_err(map)?;
            persist(actor, &mut state, &mut expected_tail).await?;
            return view(&state, false, expected_tail);
        }
        if !request.defer_reductions && !cache.pending().is_empty() {
            cache.reconcile(cache.generation(), &fence, BoundaryChange::HardFold).map_err(map)?;
            state.cache = cache.checkpoint().map_err(map)?;
            persist(actor, &mut state, &mut expected_tail).await?;
            return view(&state, false, expected_tail);
        }
        validate_tail(actor, expected_tail).await?;
        return view(&state, true, expected_tail);
    }
    let cache = ContextCache::new(fence, CacheRegion::default(), CacheRegion::default(), region(history, &request, &[])?).map_err(map)?;
    let mut state = State { version: CONTRACT_VERSION, revision: 0, history: history.export_canonical().map_err(map)?, request, cache: cache.checkpoint().map_err(map)?, summaries: Vec::new() };
    persist(actor, &mut state, &mut expected_tail).await?;
    view(&state, false, expected_tail)
}
pub async fn publish_summary(actor: &ActorEngine, history: &SourceHistory, expected_fence: &CacheFence, chunk: SourceChunk, result: HistorianResult, mut expected_tail: LSN) -> Result<ProjectionView, Error> {
    let _guard = WRITER.get_or_init(|| tokio::sync::Mutex::new(())).lock().await;
    validate_tail(actor, expected_tail).await?;
    let mut state = load(actor, history.scope(), history.session_id()).await?.ok_or_else(invalid)?;
    if &state.cache.fence != expected_fence || source_revision(history).map_err(map)? != expected_fence.source_revision || state.cache.invalidation.is_some() { return Err(invalid()); }
    let publication = Publication { chunk, result };
    validate_publication(history, &publication).map_err(map)?;
    if let Some(existing) = state.summaries.iter().find(|p| p.chunk.digest == publication.chunk.digest) { validate_tail(actor, expected_tail).await?; return if existing == &publication { view(&state, true, expected_tail) } else { Err(invalid()) }; }
    state.summaries.push(publication);
    queue_materialization(&mut state, history)?;
    persist(actor, &mut state, &mut expected_tail).await?;
    view(&state, false, expected_tail)
}
pub async fn invalidate(actor: &ActorEngine, scope: &Scope, session: &str, reason: SafetyInvalidation) -> Result<u64, Error> {
    let _guard = WRITER.get_or_init(|| tokio::sync::Mutex::new(())).lock().await;
    let mut expected_tail = actor.stats().await?.applied.last_lsn;
    let mut state = load(actor, scope, session).await?.ok_or_else(invalid)?;
    let mut cache = ContextCache::restore(state.cache.clone(), &state.cache.fence).map_err(map)?;
    let generation = cache.invalidate(reason).map_err(map)?;
    state.cache = cache.checkpoint().map_err(map)?;
    persist(actor, &mut state, &mut expected_tail).await?;
    Ok(generation)
}
pub async fn expand(actor: &ActorEngine, scope: &Scope, session: &str, span: &SourceSpan) -> Result<Vec<u8>, Error> {
    let state = load(actor, scope, session).await?.ok_or_else(invalid)?;
    if state.cache.invalidation.is_some() { return Err(invalid()); }
    SourceHistory::restore_canonical(&state.history).map_err(map)?.recover(scope, span).map_err(map)
}
pub async fn sync_summaries(actor: &ActorEngine, history: &SourceHistory, expected_fence: &CacheFence, results: Vec<(SourceChunk, HistorianResult)>, mut expected_tail: LSN) -> Result<ProjectionView, Error> {
    let _guard = WRITER.get_or_init(|| tokio::sync::Mutex::new(())).lock().await;
    validate_tail(actor, expected_tail).await?;
    let mut state = load(actor, history.scope(), history.session_id()).await?.ok_or_else(invalid)?;
    if &state.cache.fence != expected_fence || source_revision(history).map_err(map)? != expected_fence.source_revision || state.cache.invalidation.is_some() { return Err(invalid()); }
    let mut publications: Vec<_> = results.into_iter().map(|(chunk,result)| Publication { chunk, result }).collect();
    publications.sort_by(|a,b| a.chunk.digest.cmp(&b.chunk.digest));
    let mut ids = BTreeSet::new();
    for publication in &publications { validate_publication(history, publication).map_err(map)?; if !ids.insert(&publication.chunk.digest) { return Err(invalid()); } }
    let mut old = state.summaries.clone(); old.sort_by(|a,b| a.chunk.digest.cmp(&b.chunk.digest));
    if old == publications { validate_tail(actor, expected_tail).await?; return view(&state, true, expected_tail); }
    state.summaries = publications;
    queue_materialization(&mut state, history)?;
    persist(actor, &mut state, &mut expected_tail).await?;
    view(&state, false, expected_tail)
}
