#![allow(clippy::missing_errors_doc)]

use crate::generation::{decode, encode, should_apply, staged_marker_key, verify_frame};
use crate::store::{Mutation, ProjectionId, ProjectionStore, ReadSnapshot};
use hm_core::{Error, ErrorCode};
use hm_ledger::frame::{EventKind as LedgerEventKind, Frame};
use hm_schema::events::{
    ConsolidationBudget, ConsolidationOpened, ConsolidationPhase, EventPayload, PromptVersion,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

const ACTIVE_KEY: &[u8] = b"A";
const RUN_PREFIX: u8 = b'R';
const GENERATION_PREFIX: u8 = b'G';

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum RunStatus {
    Open,
    Published,
    Retracted,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum StagedProjection {
    Memories,
    Graph,
    Fsrs,
}

impl StagedProjection {
    const fn projection_id(self) -> ProjectionId {
        match self {
            Self::Memories => ProjectionId::Memories,
            Self::Graph => ProjectionId::Graph,
            Self::Fsrs => ProjectionId::Fsrs,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PromptRecord {
    pub prompt_id: String,
    pub version: u16,
    pub model_id: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct BudgetRecord {
    pub max_llm_calls: u64,
    pub max_tokens: u64,
    pub max_microusd: u64,
    pub max_wall_ms: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PhaseRecord {
    pub phase: u8,
    pub state: u8,
    pub attempt_prefix: Vec<u8>,
    pub cursor: Option<Vec<u8>>,
    pub llm_calls: u64,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cost_microusd: u64,
    pub dropped_candidates: u64,
    pub event_lsn: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RunRecord {
    pub run_id: Vec<u8>,
    pub generation: u64,
    pub parent_generation: u64,
    pub scope_digest: Vec<u8>,
    pub cadence_key: String,
    pub phases: Vec<u8>,
    pub prompts: Vec<PromptRecord>,
    pub budget: BudgetRecord,
    pub status: RunStatus,
    pub opened_lsn: u64,
    pub published_lsn: u64,
    pub retracted_lsn: u64,
    pub staged: BTreeMap<u64, StagedProjection>,
    pub progress: BTreeMap<u8, PhaseRecord>,
    pub derived_records: u64,
    pub dropped_candidates: u64,
    pub llm_calls: u64,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cost_microusd: u64,
    pub cleanup_scheduled: bool,
}

pub struct RunsProjection;

impl RunsProjection {
    #[allow(clippy::too_many_lines)]
    pub fn apply_event(store: &ProjectionStore, frame: &Frame) -> Result<(), Error> {
        let snapshot = store.begin_snapshot()?;
        if !should_apply(&snapshot, ProjectionId::Runs, frame)? {
            return Ok(());
        }
        let mut mutations = Vec::new();
        match frame.header.kind {
            LedgerEventKind::ConsolidationOpened => {
                let verified = verify_frame(frame)?;
                let EventPayload::ConsolidationOpened(opened) = verified.envelope.payload else {
                    return Err(Error::new(ErrorCode::InvalidKind).at_lsn(frame.header.lsn));
                };
                let run_id = required_run_id(verified.envelope.run_id.as_deref())?;
                apply_opened(&snapshot, frame, run_id, &opened, &mut mutations)?;
            }
            LedgerEventKind::ConsolidationPhase => {
                let verified = verify_frame(frame)?;
                let EventPayload::ConsolidationPhase(phase) = verified.envelope.payload else {
                    return Err(Error::new(ErrorCode::InvalidKind).at_lsn(frame.header.lsn));
                };
                let run_id = required_run_id(verified.envelope.run_id.as_deref())?;
                apply_phase(&snapshot, frame, run_id, &phase, &mut mutations)?;
            }
            LedgerEventKind::ConsolidationClosed => {
                let verified = verify_frame(frame)?;
                let EventPayload::ConsolidationClosed(closed) = verified.envelope.payload else {
                    return Err(Error::new(ErrorCode::InvalidKind).at_lsn(frame.header.lsn));
                };
                let run_id = required_run_id(verified.envelope.run_id.as_deref())?;
                let mut run = Self::run(&snapshot, run_id)?.ok_or_else(|| {
                    Error::new(ErrorCode::OrderingViolation).at_lsn(frame.header.lsn)
                })?;
                if run.generation != closed.generation
                    || run.parent_generation != closed.expected_active_generation
                    || closed.derived_records != run.staged.len() as u64
                {
                    return Err(Error::new(ErrorCode::IdempotencyConflict).at_lsn(frame.header.lsn));
                }
                verify_staged(&snapshot, &run)?;
                let active = Self::active_generation(&snapshot)?;
                if active != run.parent_generation && active != run.generation {
                    return Err(Error::new(ErrorCode::IdempotencyConflict).at_lsn(frame.header.lsn));
                }
                run.status = RunStatus::Published;
                run.published_lsn = run.published_lsn.max(frame.header.lsn.get());
                run.derived_records = closed.derived_records;
                run.dropped_candidates = closed.dropped_candidates;
                run.llm_calls = closed.llm_calls;
                run.input_tokens = closed.input_tokens;
                run.output_tokens = closed.output_tokens;
                run.cost_microusd = closed.cost_microusd;
                mutations.push(Mutation::put(ACTIVE_KEY, run.generation.to_le_bytes()));
                mutations.push(Mutation::put(run_key(run_id)?, encode(&run)?));
            }
            LedgerEventKind::ConsolidationRetracted => {
                let verified = verify_frame(frame)?;
                let EventPayload::ConsolidationRetracted(retracted) = verified.envelope.payload
                else {
                    return Err(Error::new(ErrorCode::InvalidKind).at_lsn(frame.header.lsn));
                };
                let mut run = Self::run(&snapshot, &retracted.target_run_id)?.ok_or_else(|| {
                    Error::new(ErrorCode::OrderingViolation).at_lsn(frame.header.lsn)
                })?;
                if run.parent_generation != retracted.previous_generation {
                    return Err(Error::new(ErrorCode::IdempotencyConflict).at_lsn(frame.header.lsn));
                }
                let active = Self::active_generation(&snapshot)?;
                if active != run.generation
                    && !(active == run.parent_generation && run.status == RunStatus::Retracted)
                {
                    return Err(Error::new(ErrorCode::IdempotencyConflict).at_lsn(frame.header.lsn));
                }
                run.status = RunStatus::Retracted;
                run.retracted_lsn = frame.header.lsn.get();
                run.cleanup_scheduled = true;
                mutations.push(Mutation::put(
                    ACTIVE_KEY,
                    run.parent_generation.to_le_bytes(),
                ));
                mutations.push(Mutation::put(
                    run_key(&retracted.target_run_id)?,
                    encode(&run)?,
                ));
            }
            kind => {
                if let Some(projection) = staged_projection(kind) {
                    let verified = verify_frame(frame)?;
                    let run_id = required_run_id(verified.envelope.run_id.as_deref())?;
                    let mut run = Self::run(&snapshot, run_id)?.ok_or_else(|| {
                        Error::new(ErrorCode::OrderingViolation).at_lsn(frame.header.lsn)
                    })?;
                    if run.status != RunStatus::Open {
                        return Err(
                            Error::new(ErrorCode::IdempotencyConflict).at_lsn(frame.header.lsn)
                        );
                    }
                    run.staged.insert(frame.header.lsn.get(), projection);
                    mutations.push(Mutation::put(run_key(run_id)?, encode(&run)?));
                }
            }
        }
        drop(snapshot);
        store.apply(ProjectionId::Runs, frame.header.lsn, &mutations)
    }

    pub fn active_generation(snapshot: &ReadSnapshot<'_>) -> Result<u64, Error> {
        snapshot
            .get(ProjectionId::Runs, ACTIVE_KEY)?
            .map_or(Ok(0), |bytes| decode_u64(&bytes))
    }

    pub fn run(snapshot: &ReadSnapshot<'_>, run_id: &[u8]) -> Result<Option<RunRecord>, Error> {
        snapshot
            .get(ProjectionId::Runs, &run_key(run_id)?)?
            .map(|bytes| decode(&bytes))
            .transpose()
    }

    pub fn run_for_generation(
        snapshot: &ReadSnapshot<'_>,
        generation: u64,
    ) -> Result<Option<RunRecord>, Error> {
        if generation == 0 {
            return Ok(None);
        }
        let Some(run_id) = snapshot.get(ProjectionId::Runs, &generation_key(generation))? else {
            return Ok(None);
        };
        Self::run(snapshot, &run_id)
    }

    pub fn generation_for_run(snapshot: &ReadSnapshot<'_>, run_id: &[u8]) -> Result<u64, Error> {
        Self::run(snapshot, run_id)?
            .map(|run| run.generation)
            .ok_or_else(|| Error::new(ErrorCode::OrderingViolation))
    }

    pub fn lineage(snapshot: &ReadSnapshot<'_>, generation: u64) -> Result<Vec<u64>, Error> {
        Self::ancestry(snapshot, generation, true)
    }

    pub(crate) fn lineage_for_staging(
        snapshot: &ReadSnapshot<'_>,
        generation: u64,
    ) -> Result<Vec<u64>, Error> {
        Self::ancestry(snapshot, generation, false)
    }

    fn ancestry(
        snapshot: &ReadSnapshot<'_>,
        generation: u64,
        published_only: bool,
    ) -> Result<Vec<u64>, Error> {
        let mut output = Vec::new();
        let mut seen = BTreeSet::new();
        let mut cursor = generation;
        while cursor != 0 {
            if !seen.insert(cursor) {
                return Err(Error::new(ErrorCode::InvariantViolation));
            }
            let run = Self::run_for_generation(snapshot, cursor)?
                .ok_or_else(|| Error::new(ErrorCode::InvariantViolation))?;
            if run.status == RunStatus::Published
                || (!published_only && run.status == RunStatus::Open)
            {
                output.push(cursor);
            }
            cursor = run.parent_generation;
        }
        output.push(0);
        Ok(output)
    }

    pub fn is_published(snapshot: &ReadSnapshot<'_>, generation: u64) -> Result<bool, Error> {
        if generation == 0 {
            return Ok(true);
        }
        Ok(Self::run_for_generation(snapshot, generation)?
            .is_some_and(|run| run.status == RunStatus::Published))
    }

    pub fn is_readable(snapshot: &ReadSnapshot<'_>, generation: u64) -> Result<bool, Error> {
        if generation == 0 {
            return Ok(true);
        }
        Ok(Self::run_for_generation(snapshot, generation)?
            .is_some_and(|run| run.status != RunStatus::Open))
    }
}

fn apply_opened(
    snapshot: &ReadSnapshot<'_>,
    frame: &Frame,
    run_id: &[u8],
    opened: &ConsolidationOpened,
    mutations: &mut Vec<Mutation>,
) -> Result<(), Error> {
    let record = opened_record(frame, run_id, opened);
    if let Some(existing) = RunsProjection::run(snapshot, run_id)? {
        if !same_opening(&existing, &record) {
            return Err(Error::new(ErrorCode::IdempotencyConflict).at_lsn(frame.header.lsn));
        }
        return Ok(());
    }
    if snapshot
        .get(ProjectionId::Runs, &generation_key(opened.generation))?
        .is_some()
    {
        return Err(Error::new(ErrorCode::IdempotencyConflict).at_lsn(frame.header.lsn));
    }
    if RunsProjection::active_generation(snapshot)? != opened.expected_active_generation {
        return Err(Error::new(ErrorCode::IdempotencyConflict).at_lsn(frame.header.lsn));
    }
    mutations.push(Mutation::put(generation_key(opened.generation), run_id));
    mutations.push(Mutation::put(run_key(run_id)?, encode(&record)?));
    Ok(())
}

fn apply_phase(
    snapshot: &ReadSnapshot<'_>,
    frame: &Frame,
    run_id: &[u8],
    phase: &ConsolidationPhase,
    mutations: &mut Vec<Mutation>,
) -> Result<(), Error> {
    let mut run = RunsProjection::run(snapshot, run_id)?
        .ok_or_else(|| Error::new(ErrorCode::OrderingViolation).at_lsn(frame.header.lsn))?;
    if run.status != RunStatus::Open {
        return Err(Error::new(ErrorCode::IdempotencyConflict).at_lsn(frame.header.lsn));
    }
    let progress = PhaseRecord {
        phase: phase.phase as u8,
        state: phase.state as u8,
        attempt_prefix: phase.attempt_prefix.clone(),
        cursor: phase.cursor.clone(),
        llm_calls: phase.llm_calls,
        input_tokens: phase.input_tokens,
        output_tokens: phase.output_tokens,
        cost_microusd: phase.cost_microusd,
        dropped_candidates: phase.dropped_candidates,
        event_lsn: frame.header.lsn.get(),
    };
    if let Some(existing) = run.progress.get(&(phase.phase as u8))
        && existing.attempt_prefix == progress.attempt_prefix
    {
        if same_phase(existing, &progress) {
            return Ok(());
        }
        return Err(Error::new(ErrorCode::IdempotencyConflict).at_lsn(frame.header.lsn));
    }
    run.progress.insert(phase.phase as u8, progress);
    mutations.push(Mutation::put(run_key(run_id)?, encode(&run)?));
    Ok(())
}

fn opened_record(frame: &Frame, run_id: &[u8], opened: &ConsolidationOpened) -> RunRecord {
    RunRecord {
        run_id: run_id.to_vec(),
        generation: opened.generation,
        parent_generation: opened.expected_active_generation,
        scope_digest: opened.scope_digest.clone(),
        cadence_key: opened.cadence_key.clone(),
        phases: opened.phases.iter().map(|phase| *phase as u8).collect(),
        prompts: opened.prompts.iter().map(prompt_record).collect(),
        budget: budget_record(&opened.budget),
        status: RunStatus::Open,
        opened_lsn: frame.header.lsn.get(),
        published_lsn: 0,
        retracted_lsn: 0,
        staged: BTreeMap::new(),
        progress: BTreeMap::new(),
        derived_records: 0,
        dropped_candidates: 0,
        llm_calls: 0,
        input_tokens: 0,
        output_tokens: 0,
        cost_microusd: 0,
        cleanup_scheduled: false,
    }
}

fn prompt_record(prompt: &PromptVersion) -> PromptRecord {
    PromptRecord {
        prompt_id: prompt.prompt_id.clone(),
        version: prompt.version,
        model_id: prompt.model_id.clone(),
    }
}

const fn budget_record(budget: &ConsolidationBudget) -> BudgetRecord {
    BudgetRecord {
        max_llm_calls: budget.max_llm_calls,
        max_tokens: budget.max_tokens,
        max_microusd: budget.max_microusd,
        max_wall_ms: budget.max_wall_ms,
    }
}

fn same_opening(left: &RunRecord, right: &RunRecord) -> bool {
    left.run_id == right.run_id
        && left.generation == right.generation
        && left.parent_generation == right.parent_generation
        && left.scope_digest == right.scope_digest
        && left.cadence_key == right.cadence_key
        && left.phases == right.phases
        && left.prompts == right.prompts
        && left.budget == right.budget
}

fn same_phase(left: &PhaseRecord, right: &PhaseRecord) -> bool {
    left.phase == right.phase
        && left.state == right.state
        && left.attempt_prefix == right.attempt_prefix
        && left.cursor == right.cursor
        && left.llm_calls == right.llm_calls
        && left.input_tokens == right.input_tokens
        && left.output_tokens == right.output_tokens
        && left.cost_microusd == right.cost_microusd
        && left.dropped_candidates == right.dropped_candidates
}

fn verify_staged(snapshot: &ReadSnapshot<'_>, run: &RunRecord) -> Result<(), Error> {
    for (lsn, projection) in &run.staged {
        let projection = projection.projection_id();
        if snapshot.checkpoint(projection)?.get() < *lsn
            || snapshot
                .get(projection, &staged_marker_key(*lsn))?
                .is_none()
        {
            return Err(Error::new(ErrorCode::ProjectionCheckpoint).at_offset(*lsn));
        }
    }
    Ok(())
}

const fn staged_projection(kind: LedgerEventKind) -> Option<StagedProjection> {
    match kind {
        LedgerEventKind::MemoryMinted
        | LedgerEventKind::MemoryRevised
        | LedgerEventKind::MemoryMerged
        | LedgerEventKind::MemoryFaded => Some(StagedProjection::Memories),
        LedgerEventKind::EdgeAsserted | LedgerEventKind::EdgeRetracted => {
            Some(StagedProjection::Graph)
        }
        LedgerEventKind::Reviewed => Some(StagedProjection::Fsrs),
        _ => None,
    }
}

fn required_run_id(run_id: Option<&[u8]>) -> Result<&[u8], Error> {
    run_id
        .filter(|value| !value.is_empty())
        .ok_or_else(|| Error::new(ErrorCode::SchemaInvalid))
}

fn run_key(run_id: &[u8]) -> Result<Vec<u8>, Error> {
    if run_id.is_empty() {
        return Err(Error::new(ErrorCode::InvalidArgument));
    }
    let length = u16::try_from(run_id.len()).map_err(|_| Error::new(ErrorCode::InvalidLength))?;
    let mut key = Vec::with_capacity(run_id.len() + 3);
    key.push(RUN_PREFIX);
    key.extend_from_slice(&length.to_be_bytes());
    key.extend_from_slice(run_id);
    Ok(key)
}

fn generation_key(generation: u64) -> [u8; 9] {
    let mut key = [0_u8; 9];
    key[0] = GENERATION_PREFIX;
    key[1..].copy_from_slice(&generation.to_be_bytes());
    key
}

fn decode_u64(bytes: &[u8]) -> Result<u64, Error> {
    bytes
        .try_into()
        .map(u64::from_le_bytes)
        .map_err(|_| Error::new(ErrorCode::InvariantViolation))
}
