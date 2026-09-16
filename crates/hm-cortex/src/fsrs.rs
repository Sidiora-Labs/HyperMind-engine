#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::missing_errors_doc
)]

use hm_schema::events::{ReviewRating, Reviewed};

pub const FSRS_WEIGHTS: [f64; 21] = [
    0.4072, 1.1829, 3.1262, 15.4722, 7.2102, 0.5316, 1.0651, 0.0589, 1.506, 0.14, 1.0036, 1.9395,
    0.11, 0.2918, 0.5, 1.0, 2.0, 0.0, 0.0, 0.0, 0.0,
];
pub const DESIRED_RETENTION: f64 = 0.9;
const DECAY: f64 = -0.5;
const NANOS_PER_DAY: i64 = 86_400_000_000_000;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FsrsState {
    New,
    Learning,
    Review,
    Relearning,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FsrsData {
    pub stability: f64,
    pub difficulty: f64,
    pub repetitions: u32,
    pub lapses: u32,
    pub state: FsrsState,
    pub last_review_ns: Option<i64>,
}

impl Default for FsrsData {
    fn default() -> Self {
        Self {
            stability: FSRS_WEIGHTS[2],
            difficulty: FSRS_WEIGHTS[4],
            repetitions: 0,
            lapses: 0,
            state: FsrsState::New,
            last_review_ns: None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScheduleResult {
    pub stability: f64,
    pub difficulty: f64,
    pub interval_days: u32,
    pub state: FsrsState,
    pub repetitions: u32,
    pub lapses: u32,
}

#[must_use]
pub fn retrievability(stability: f64, elapsed_days: f64) -> f64 {
    if !stability.is_finite() || stability <= 0.0 || !elapsed_days.is_finite() {
        return 0.0;
    }
    let factor = DESIRED_RETENTION.powf(1.0 / DECAY) - 1.0;
    (1.0 + factor * elapsed_days.max(0.0) / stability).powf(DECAY)
}

#[must_use]
pub fn initial_stability(rating: ReviewRating) -> f64 {
    FSRS_WEIGHTS[rating_index(rating)].max(0.1)
}

#[must_use]
pub fn schedule_next(current: FsrsData, rating: ReviewRating, elapsed_days: f64) -> ScheduleResult {
    let ordinal = rating_index(rating) as f64 + 1.0;
    let (stability, difficulty, state, lapses) = if current.state == FsrsState::New {
        (
            initial_stability(rating),
            FSRS_WEIGHTS[4] - FSRS_WEIGHTS[5] * (ordinal - 3.0),
            if rating == ReviewRating::Again {
                FsrsState::Relearning
            } else {
                FsrsState::Learning
            },
            u32::from(rating == ReviewRating::Again),
        )
    } else {
        let recall = retrievability(current.stability, elapsed_days);
        let difficulty = update_difficulty(current.difficulty, ordinal);
        if rating == ReviewRating::Again {
            (
                stability_after_forgetting(difficulty, current.stability, recall),
                difficulty,
                FsrsState::Relearning,
                current.lapses.saturating_add(1),
            )
        } else {
            (
                stability_after_recall(difficulty, current.stability, recall, rating),
                difficulty,
                FsrsState::Review,
                current.lapses,
            )
        }
    };
    let interval = (stability * DESIRED_RETENTION.ln() / 0.9_f64.ln())
        .round()
        .max(1.0);
    ScheduleResult {
        stability,
        difficulty: difficulty.clamp(1.0, 10.0),
        interval_days: interval.min(f64::from(u32::MAX)) as u32,
        state,
        repetitions: current.repetitions.saturating_add(1),
        lapses,
    }
}

#[must_use]
pub fn reviewed_event(
    memory_id: Vec<u8>,
    source_lsn: u64,
    reviewed_at_ns: i64,
    current: FsrsData,
    rating: ReviewRating,
) -> (Reviewed, FsrsData) {
    let elapsed_days = current.last_review_ns.map_or(0.0, |last| {
        reviewed_at_ns.saturating_sub(last).max(0) as f64 / NANOS_PER_DAY as f64
    });
    let scheduled = schedule_next(current, rating, elapsed_days);
    let interval_ns = i64::from(scheduled.interval_days).saturating_mul(NANOS_PER_DAY);
    let event = Reviewed {
        memory_id,
        rating,
        source_lsn,
        reviewed_at_ns,
        stability_millis: float_to_u64(scheduled.stability * 1_000.0),
        difficulty_micros: float_to_u32(scheduled.difficulty * 1_000_000.0),
        due_at_ns: reviewed_at_ns.saturating_add(interval_ns),
    };
    (
        event,
        FsrsData {
            stability: scheduled.stability,
            difficulty: scheduled.difficulty,
            repetitions: scheduled.repetitions,
            lapses: scheduled.lapses,
            state: scheduled.state,
            last_review_ns: Some(reviewed_at_ns),
        },
    )
}

fn update_difficulty(difficulty: f64, ordinal: f64) -> f64 {
    let delta = -FSRS_WEIGHTS[6] * (ordinal - 3.0);
    let mean_reversion = FSRS_WEIGHTS[7] * (FSRS_WEIGHTS[4] - difficulty);
    (difficulty + delta + mean_reversion).clamp(1.0, 10.0)
}

fn stability_after_recall(
    difficulty: f64,
    stability: f64,
    recall: f64,
    rating: ReviewRating,
) -> f64 {
    let hard_penalty = if rating == ReviewRating::Hard {
        FSRS_WEIGHTS[15]
    } else {
        1.0
    };
    let easy_bonus = if rating == ReviewRating::Easy {
        FSRS_WEIGHTS[16]
    } else {
        1.0
    };
    stability
        * (FSRS_WEIGHTS[8].exp()
            * (11.0 - difficulty)
            * stability.powf(-FSRS_WEIGHTS[9])
            * (((1.0 - recall) * FSRS_WEIGHTS[10]).exp() - 1.0)
            * hard_penalty
            * easy_bonus)
        + stability
}

fn stability_after_forgetting(difficulty: f64, stability: f64, recall: f64) -> f64 {
    FSRS_WEIGHTS[11]
        * difficulty.powf(-FSRS_WEIGHTS[12])
        * ((stability + 1.0).powf(FSRS_WEIGHTS[13]) - 1.0)
        * ((1.0 - recall) * FSRS_WEIGHTS[14]).exp()
}

const fn rating_index(rating: ReviewRating) -> usize {
    match rating {
        ReviewRating::Again => 0,
        ReviewRating::Hard => 1,
        ReviewRating::Good => 2,
        ReviewRating::Easy => 3,
    }
}

fn float_to_u64(value: f64) -> u64 {
    if value.is_finite() && value > 0.0 {
        value.round().min(u64::MAX as f64) as u64
    } else {
        0
    }
}

fn float_to_u32(value: f64) -> u32 {
    if value.is_finite() && value > 0.0 {
        value.round().min(f64::from(u32::MAX)) as u32
    } else {
        0
    }
}
