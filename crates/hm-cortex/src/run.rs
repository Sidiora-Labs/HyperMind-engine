#![allow(clippy::missing_errors_doc)]

use crate::budget::BudgetUsage;
use chrono::{DateTime, Days, LocalResult, NaiveDate, NaiveTime, TimeZone, Utc};
use chrono_tz::Tz;
use hm_schema::events::{
    ConsolidationPhase, ConsolidationPhaseName, ConsolidationPhaseState, ConsolidationRetracted,
};
use std::collections::BTreeMap;

#[must_use]
pub fn run_id(scope: &[u8], cadence_key: &str, generation: u64) -> [u8; 32] {
    let mut hash = blake3::Hasher::new();
    hash.update(b"hypermind consolidation run v1\0");
    hash.update(&(scope.len() as u64).to_le_bytes());
    hash.update(scope);
    hash.update(&(cadence_key.len() as u64).to_le_bytes());
    hash.update(cadence_key.as_bytes());
    hash.update(&generation.to_le_bytes());
    *hash.finalize().as_bytes()
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PersistedPhase {
    pub phase: ConsolidationPhaseName,
    pub state: ConsolidationPhaseState,
    pub attempt: u32,
    pub cursor: Option<Vec<u8>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PhaseWork {
    pub phase: ConsolidationPhaseName,
    pub attempt: u32,
    pub cursor: Option<Vec<u8>>,
    pub resumed: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PhaseMachine {
    run_id: Vec<u8>,
    phases: Vec<ConsolidationPhaseName>,
    progress: BTreeMap<u8, PersistedPhase>,
}

impl PhaseMachine {
    #[must_use]
    pub fn resume(
        run_id: Vec<u8>,
        phases: Vec<ConsolidationPhaseName>,
        persisted: impl IntoIterator<Item = PersistedPhase>,
    ) -> Self {
        let progress = persisted
            .into_iter()
            .map(|phase| (phase.phase as u8, phase))
            .collect();
        Self {
            run_id,
            phases,
            progress,
        }
    }

    #[must_use]
    pub fn next(&self) -> Option<PhaseWork> {
        self.phases.iter().find_map(|phase| {
            let existing = self.progress.get(&(*phase as u8));
            match existing.map(|value| value.state) {
                Some(ConsolidationPhaseState::Completed) => None,
                Some(ConsolidationPhaseState::Started) => Some(PhaseWork {
                    phase: *phase,
                    attempt: existing.map_or(1, |value| value.attempt),
                    cursor: existing.and_then(|value| value.cursor.clone()),
                    resumed: true,
                }),
                Some(ConsolidationPhaseState::Aborted) => Some(PhaseWork {
                    phase: *phase,
                    attempt: existing.map_or(1, |value| value.attempt.saturating_add(1)),
                    cursor: existing.and_then(|value| value.cursor.clone()),
                    resumed: false,
                }),
                None => Some(PhaseWork {
                    phase: *phase,
                    attempt: 1,
                    cursor: None,
                    resumed: false,
                }),
            }
        })
    }

    #[must_use]
    pub fn event(
        &self,
        work: &PhaseWork,
        state: ConsolidationPhaseState,
        cursor: Option<Vec<u8>>,
        usage: BudgetUsage,
        dropped_candidates: u64,
    ) -> ConsolidationPhase {
        ConsolidationPhase {
            phase: work.phase,
            state,
            attempt_prefix: attempt_prefix(&self.run_id, work.phase, work.attempt, state).to_vec(),
            cursor,
            llm_calls: usage.llm_calls,
            input_tokens: usage.input_tokens,
            output_tokens: usage.output_tokens,
            cost_microusd: usage.cost_microusd,
            dropped_candidates,
        }
    }

    pub fn record(&mut self, work: &PhaseWork, event: &ConsolidationPhase) {
        self.progress.insert(
            event.phase as u8,
            PersistedPhase {
                phase: event.phase,
                state: event.state,
                attempt: work.attempt,
                cursor: event.cursor.clone(),
            },
        );
    }
}

#[must_use]
pub fn retraction_event(
    target_run_id: Vec<u8>,
    previous_generation: u64,
    reason: impl Into<String>,
) -> ConsolidationRetracted {
    ConsolidationRetracted {
        target_run_id,
        previous_generation,
        reason: reason.into(),
    }
}

fn attempt_prefix(
    run_id: &[u8],
    phase: ConsolidationPhaseName,
    attempt: u32,
    state: ConsolidationPhaseState,
) -> [u8; 16] {
    let mut hash = blake3::Hasher::new();
    hash.update(b"hypermind phase attempt v1\0");
    hash.update(run_id);
    hash.update(&[phase as u8]);
    hash.update(&attempt.to_le_bytes());
    let mut output = [0; 16];
    output[..12].copy_from_slice(&hash.finalize().as_bytes()[..12]);
    let state_hash = blake3::hash(&[state as u8]);
    output[12..].copy_from_slice(&state_hash.as_bytes()[..4]);
    output
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Cadence {
    pub every_days: u32,
    pub local_hour: u8,
    pub local_minute: u8,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CadenceError {
    UnknownTimezone,
    InvalidCadence,
    TimestampOutOfRange,
}

pub fn due(
    last_completed_ns: Option<i64>,
    now_ns: i64,
    iana_timezone: &str,
    cadence: Cadence,
) -> Result<bool, CadenceError> {
    if cadence.every_days == 0 || cadence.local_hour > 23 || cadence.local_minute > 59 {
        return Err(CadenceError::InvalidCadence);
    }
    let timezone = iana_timezone
        .parse::<Tz>()
        .map_err(|_| CadenceError::UnknownTimezone)?;
    let now = DateTime::<Utc>::from_timestamp_nanos(now_ns).with_timezone(&timezone);
    let next_date = if let Some(last) = last_completed_ns {
        DateTime::<Utc>::from_timestamp_nanos(last)
            .with_timezone(&timezone)
            .date_naive()
            .checked_add_days(Days::new(u64::from(cadence.every_days)))
            .ok_or(CadenceError::TimestampOutOfRange)?
    } else {
        now.date_naive()
    };
    let scheduled = local_schedule(
        timezone,
        next_date,
        cadence.local_hour,
        cadence.local_minute,
    )?;
    Ok(now >= scheduled)
}

fn local_schedule(
    timezone: Tz,
    date: NaiveDate,
    hour: u8,
    minute: u8,
) -> Result<DateTime<Tz>, CadenceError> {
    let time = NaiveTime::from_hms_opt(u32::from(hour), u32::from(minute), 0)
        .ok_or(CadenceError::InvalidCadence)?;
    let mut local = date.and_time(time);
    for _ in 0..=120 {
        match timezone.from_local_datetime(&local) {
            LocalResult::Single(value) => return Ok(value),
            LocalResult::Ambiguous(first, second) => return Ok(first.min(second)),
            LocalResult::None => {
                local = local
                    .checked_add_signed(chrono::Duration::minutes(1))
                    .ok_or(CadenceError::TimestampOutOfRange)?;
            }
        }
    }
    Err(CadenceError::TimestampOutOfRange)
}
