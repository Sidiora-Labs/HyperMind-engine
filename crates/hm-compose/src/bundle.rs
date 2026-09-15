#![allow(clippy::missing_errors_doc)]

use crate::budget::BudgetProfile;
use crate::canonical::{bundle_hash, canonical_bytes};
use crate::lanes::lexical;
use crate::tiers::{bindings, intent, work};
use crate::tokens::TokenCounter;
use crate::trim::trim_to_budget_with_profile;
use hm_core::{ActorId, ConversationId, Error, ErrorCode, LSN, UtcNanos};
use hm_ledger::frame::{EventKind, Frame, FrameHeader};
use hm_proj::store::{ProjectionId, ReadSnapshot};
use hm_proj::timeline::{ConversationRecord, read_conversation_record, read_conversation_records};
use hm_schema::event::{self, Boundary, encode_event_envelope};
use hm_schema::events::{
    Attestation, AttestationDisposition, Authority, EventEnvelope, EventPayload, Retention,
    Sensitivity,
};
use std::collections::BTreeSet;

pub use crate::trim::trim_to_budget;

pub const MAXIMUM_CANDIDATES: usize = 4096;
pub const MAXIMUM_CONVERSATION_RECORDS: usize = 16_384;
pub const MAXIMUM_QUERY_BYTES: usize = 1024 * 1024;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
#[repr(u8)]
pub enum Tier {
    Resident,
    Intent,
    Bindings,
    WorkLedger,
    Prospective,
    Conversation,
    Entity,
    Conflicts,
    Fused,
    Temporal,
}

impl Tier {
    pub const ALL: [Self; 10] = [
        Self::Resident,
        Self::Intent,
        Self::Bindings,
        Self::WorkLedger,
        Self::Prospective,
        Self::Conversation,
        Self::Entity,
        Self::Conflicts,
        Self::Fused,
        Self::Temporal,
    ];

    #[must_use]
    pub const fn required(self) -> bool {
        matches!(
            self,
            Self::Resident | Self::Intent | Self::Bindings | Self::WorkLedger
        )
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
#[repr(u8)]
pub enum RetrievalLane {
    Lexical,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum WhyCode {
    Conversation,
    Lexical,
    Intent,
    Binding,
    WorkLedger,
}

pub struct ActivationRequest<'model> {
    pub actor: ActorId,
    pub conversation: ConversationId,
    pub query: String,
    pub turn_text: String,
    pub budget_tokens: usize,
    pub token_counter: &'model TokenCounter,
    pub maximum_candidates: usize,
    pub maximum_conversation_records: usize,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ActivationContext {
    pub task: Option<Vec<u8>>,
    pub required_bindings: Vec<hm_proj::bindings::BindingRequirement>,
    pub now_ns: Option<UtcNanos>,
    pub budget_profile: BudgetProfile,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActivationItem {
    pub tier: Tier,
    pub uri: String,
    pub provenance: Vec<LSN>,
    pub content: Vec<u8>,
    pub tokens: usize,
    pub coarsened: bool,
    pub vector_rank: u32,
    pub lexical_rank: u32,
    pub why: WhyCode,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActivationSection {
    pub tier: Tier,
    pub required: bool,
    pub items: Vec<ActivationItem>,
    pub tokens: usize,
    pub trimmed_items: usize,
    pub coarsened_items: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum GapKind {
    TruncatedLane,
    DroppedTier,
    NarrowedSubtask,
    MissingBinding,
    StaleBinding,
    ConflictingBinding,
    IndexLag,
    PendingProtectedProposal,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Gap {
    pub kind: GapKind,
    pub tier: Option<Tier>,
    pub lane: Option<RetrievalLane>,
    pub detail: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum HealthStatus {
    SemanticReady,
    SemanticLagging,
    LexicalOnly,
    Unavailable,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BundleHealth {
    pub encoder: HealthStatus,
    pub backlog: HealthStatus,
    pub projection: HealthStatus,
    pub inclusion: HealthStatus,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RetrievalManifest {
    pub query_digest: [u8; 32],
    pub snapshot_epoch: u64,
    pub encoder: String,
    pub index_generation: u64,
    pub candidate_lanes: Vec<RetrievalLane>,
    pub candidates: Vec<LSN>,
    pub selected: Vec<LSN>,
    pub included: Vec<LSN>,
    pub used: Vec<LSN>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActivationBundle {
    pub snapshot_epoch: u64,
    pub budget_tokens: usize,
    pub spent_tokens: usize,
    pub sections: [ActivationSection; 10],
    pub manifest: RetrievalManifest,
    pub gaps: Vec<Gap>,
    pub health: BundleHealth,
    pub bundle_hash: [u8; 32],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AttestationRequest {
    pub first_lsn: LSN,
    pub actor: ActorId,
    pub conversation: ConversationId,
    pub wall_timestamp_ns: UtcNanos,
    pub disposition: AttestationDisposition,
}

pub fn activate(
    snapshot: &ReadSnapshot<'_>,
    request: &ActivationRequest<'_>,
) -> Result<ActivationBundle, Error> {
    activate_with_context(snapshot, request, &ActivationContext::default())
}

pub fn activate_with_context(
    snapshot: &ReadSnapshot<'_>,
    request: &ActivationRequest<'_>,
    context: &ActivationContext,
) -> Result<ActivationBundle, Error> {
    validate_request(request)?;
    context.budget_profile.validate()?;
    let snapshot_epoch =
        u64::try_from(snapshot.epoch()).map_err(|_| Error::new(ErrorCode::CapacityExceeded))?;
    let query_digest = *blake3::hash(request.query.as_bytes()).as_bytes();
    let mut bundle = ActivationBundle {
        snapshot_epoch,
        budget_tokens: request.budget_tokens,
        spent_tokens: 0,
        sections: std::array::from_fn(|index| ActivationSection {
            tier: Tier::ALL[index],
            required: Tier::ALL[index].required(),
            items: Vec::new(),
            tokens: 0,
            trimmed_items: 0,
            coarsened_items: 0,
        }),
        manifest: RetrievalManifest {
            query_digest,
            snapshot_epoch,
            encoder: request.token_counter.model_id().to_owned(),
            index_generation: snapshot.checkpoint(ProjectionId::Bm25)?.get(),
            candidate_lanes: vec![RetrievalLane::Lexical],
            candidates: Vec::new(),
            selected: Vec::new(),
            included: Vec::new(),
            used: Vec::new(),
        },
        gaps: Vec::new(),
        health: BundleHealth {
            encoder: HealthStatus::Unavailable,
            backlog: HealthStatus::SemanticReady,
            projection: HealthStatus::SemanticReady,
            inclusion: HealthStatus::LexicalOnly,
        },
        bundle_hash: [0; 32],
    };

    let intent = intent::read(
        snapshot,
        request.actor,
        request.conversation,
        request.token_counter,
    )?;
    let active_task = context.task.as_deref().or(intent.active_task.as_deref());
    for item in intent.items {
        add_item(&mut bundle, item)?;
    }
    let now_ns = match context.now_ns {
        Some(value) => value,
        None => latest_projection_time(snapshot)?,
    };
    let bindings = bindings::read(
        snapshot,
        request.actor,
        request.conversation,
        active_task,
        &context.required_bindings,
        now_ns,
        request.token_counter,
    )?;
    for item in bindings.items {
        add_item(&mut bundle, item)?;
    }
    bundle.gaps.extend(bindings.gaps);
    for item in work::read(
        snapshot,
        request.actor,
        request.conversation,
        request.token_counter,
    )? {
        add_item(&mut bundle, item)?;
    }
    populate_conversation(&mut bundle, snapshot, request)?;
    populate_lexical(&mut bundle, snapshot, request)?;
    trim_to_budget_with_profile(&mut bundle, request.token_counter, context.budget_profile)?;
    let included: BTreeSet<LSN> = bundle
        .sections
        .iter()
        .flat_map(|section| section.items.iter())
        .flat_map(|item| item.provenance.iter().copied())
        .collect();
    bundle.manifest.included = bundle
        .manifest
        .selected
        .iter()
        .copied()
        .filter(|lsn| included.contains(lsn))
        .collect();
    bundle.bundle_hash = bundle_hash(&bundle)?;
    Ok(bundle)
}

fn latest_projection_time(snapshot: &ReadSnapshot<'_>) -> Result<UtcNanos, Error> {
    let checkpoint = snapshot.checkpoint(ProjectionId::ConversationHeads)?;
    if checkpoint.get() == 0 {
        return Ok(UtcNanos::new(0));
    }
    read_conversation_record(snapshot, checkpoint)?
        .map(|record| record.wall_timestamp_ns)
        .ok_or_else(|| Error::new(ErrorCode::InvariantViolation).at_lsn(checkpoint))
}

pub fn build_attestations(
    bundle: &ActivationBundle,
    request: AttestationRequest,
) -> Result<Vec<Frame>, Error> {
    if request.first_lsn.get() == 0 {
        return Err(Error::new(ErrorCode::InvalidArgument));
    }
    let targets: BTreeSet<LSN> = bundle
        .sections
        .iter()
        .flat_map(|section| section.items.iter())
        .flat_map(|item| item.provenance.iter().copied())
        .collect();
    let mut frames = Vec::with_capacity(targets.len());
    for (index, target) in targets.into_iter().enumerate() {
        let lsn = request
            .first_lsn
            .get()
            .checked_add(index as u64)
            .ok_or_else(|| Error::new(ErrorCode::CapacityExceeded))?;
        let envelope = EventEnvelope {
            schema_version: 2,
            payload: EventPayload::Attestation(Box::new(Attestation {
                target_lsn: target.get(),
                disposition: request.disposition,
            })),
            connection_id: None,
            client_seq: 0,
            client_event_index: 0,
            client_event_count: 0,
            origin_actor: 0,
            run_id: None,
            model_provenance: None,
            authority: Authority::RuntimeFact,
            retention: Retention::Daily,
            sensitivity: Sensitivity::Public,
            event_time_ns: request.wall_timestamp_ns.get(),
        };
        frames.push(Frame {
            header: FrameHeader {
                lsn: LSN::new(lsn),
                kind: EventKind::Attestation,
                wall_timestamp_ns: request.wall_timestamp_ns,
                actor: request.actor,
                conversation: request.conversation,
            },
            sealed_payload: encode_event_envelope(&envelope),
        });
    }
    Ok(frames)
}

fn validate_request(request: &ActivationRequest<'_>) -> Result<(), Error> {
    if request.actor.get() == 0
        || request.query.len() > MAXIMUM_QUERY_BYTES
        || request.turn_text.len() > MAXIMUM_QUERY_BYTES
        || request.budget_tokens == 0
        || !(1..=MAXIMUM_CANDIDATES).contains(&request.maximum_candidates)
        || !(1..=MAXIMUM_CONVERSATION_RECORDS).contains(&request.maximum_conversation_records)
    {
        Err(Error::new(ErrorCode::InvalidArgument))
    } else {
        Ok(())
    }
}

fn populate_conversation(
    bundle: &mut ActivationBundle,
    snapshot: &ReadSnapshot<'_>,
    request: &ActivationRequest<'_>,
) -> Result<(), Error> {
    let requested = request
        .maximum_conversation_records
        .checked_add(1)
        .ok_or_else(|| Error::new(ErrorCode::CapacityExceeded))?;
    let mut records = read_conversation_records(snapshot, request.conversation, requested)?;
    if records.len() > request.maximum_conversation_records {
        records.remove(0);
        bundle.gaps.push(Gap {
            kind: GapKind::TruncatedLane,
            tier: Some(Tier::Conversation),
            lane: None,
            detail: "conversation record limit reached".to_owned(),
        });
    }
    for record in records {
        let Some(content) = record_content(&record)? else {
            continue;
        };
        let item = ActivationItem {
            tier: Tier::Conversation,
            uri: provenance_uri(request.actor, &record, 0, 0, WhyCode::Conversation),
            provenance: vec![record.lsn],
            tokens: request.token_counter.count(&content)?,
            content,
            coarsened: false,
            vector_rank: 0,
            lexical_rank: 0,
            why: WhyCode::Conversation,
        };
        add_item(bundle, item)?;
    }
    Ok(())
}

fn populate_lexical(
    bundle: &mut ActivationBundle,
    snapshot: &ReadSnapshot<'_>,
    request: &ActivationRequest<'_>,
) -> Result<(), Error> {
    if request.query.is_empty() {
        bundle.manifest.candidate_lanes.clear();
        return Ok(());
    }
    let query_limit = request
        .maximum_candidates
        .checked_add(1)
        .ok_or_else(|| Error::new(ErrorCode::CapacityExceeded))?;
    let mut hits = lexical::search(snapshot, &request.query, query_limit)?;
    if hits.len() > request.maximum_candidates {
        hits.truncate(request.maximum_candidates);
        bundle.gaps.push(Gap {
            kind: GapKind::TruncatedLane,
            tier: Some(Tier::Fused),
            lane: Some(RetrievalLane::Lexical),
            detail: "lexical candidate limit reached".to_owned(),
        });
    }
    bundle.manifest.candidates = hits.iter().map(|hit| hit.lsn).collect();
    bundle.manifest.selected = bundle.manifest.candidates.clone();
    for (index, hit) in hits.into_iter().enumerate() {
        let Some(record) = read_conversation_record(snapshot, hit.lsn)? else {
            continue;
        };
        if record.conversation == request.conversation {
            continue;
        }
        let Some(content) = record_content(&record)? else {
            continue;
        };
        let lexical_rank =
            u32::try_from(index + 1).map_err(|_| Error::new(ErrorCode::CapacityExceeded))?;
        let item = ActivationItem {
            tier: Tier::Fused,
            uri: provenance_uri(
                request.actor,
                &record,
                hit.score_q32,
                lexical_rank,
                WhyCode::Lexical,
            ),
            provenance: vec![record.lsn],
            tokens: request.token_counter.count(&content)?,
            content,
            coarsened: false,
            vector_rank: 0,
            lexical_rank,
            why: WhyCode::Lexical,
        };
        add_item(bundle, item)?;
    }
    Ok(())
}

fn record_content(record: &ConversationRecord) -> Result<Option<Vec<u8>>, Error> {
    let schema_kind = match record.kind {
        EventKind::UserMsg => event::EventKind::UserMsg,
        EventKind::DeliveredMsg => event::EventKind::DeliveredMsg,
        _ => return Ok(None),
    };
    let verified = event::verify_event(&record.payload, schema_kind, Boundary::Disk)?;
    match verified.envelope.payload {
        EventPayload::UserMsg(message) => Ok(Some(message.content)),
        EventPayload::DeliveredMsg(message) => Ok(Some(message.content)),
        _ => Err(Error::new(ErrorCode::InvariantViolation).at_lsn(record.lsn)),
    }
}

fn provenance_uri(
    actor: ActorId,
    record: &ConversationRecord,
    score: u64,
    lexical_rank: u32,
    why: WhyCode,
) -> String {
    let why = match why {
        WhyCode::Conversation => "conversation",
        WhyCode::Lexical => "lexical",
        WhyCode::Intent => "intent",
        WhyCode::Binding => "binding",
        WhyCode::WorkLedger => "work_ledger",
    };
    format!(
        "hm://{}/{}/{}?at={}&src={}&score={score}&vr=0&lr={lexical_rank}&why={why}",
        actor, record.conversation, record.lsn, record.wall_timestamp_ns, record.kind as u8,
    )
}

fn add_item(bundle: &mut ActivationBundle, item: ActivationItem) -> Result<(), Error> {
    if item.uri.is_empty() || item.provenance.is_empty() {
        return Err(Error::new(ErrorCode::InvalidArgument));
    }
    let section = &mut bundle.sections[item.tier as usize];
    section.tokens = section
        .tokens
        .checked_add(item.tokens)
        .ok_or_else(|| Error::new(ErrorCode::CapacityExceeded))?;
    section.items.push(item);
    Ok(())
}

pub fn canonical_and_hash(bundle: &ActivationBundle) -> Result<(Vec<u8>, [u8; 32]), Error> {
    Ok((canonical_bytes(bundle)?, bundle_hash(bundle)?))
}
