#![allow(clippy::missing_errors_doc)]

use hm_core::{Error, ErrorCode, LSN};
use hm_proj::beliefs::{BeliefAsOf, BeliefAsOfAxis, BeliefProjection, BeliefRecord};
use hm_proj::ladder::{TemporalLadder, TemporalLevel, TemporalWindow};
use hm_proj::store::ReadSnapshot;
use hm_schema::events::BeliefType;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AsOfRecall {
    pub axis: BeliefAsOfAxis,
    pub record: Option<BeliefRecord>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TimelineRecall {
    pub windows: Vec<TemporalWindow>,
    pub members: Vec<LSN>,
}

pub fn recall_asof(
    snapshot: &ReadSnapshot<'_>,
    belief_type: BeliefType,
    canonical_identity: &str,
    as_of: BeliefAsOf,
) -> Result<AsOfRecall, Error> {
    let result = BeliefProjection::read_as_of(snapshot, belief_type, canonical_identity, as_of)?;
    Ok(AsOfRecall {
        axis: result.axis,
        record: result.record,
    })
}

pub fn recall_timeline(
    snapshot: &ReadSnapshot<'_>,
    anchor_ns: i64,
    member_limit: usize,
) -> Result<TimelineRecall, Error> {
    if member_limit == 0 {
        return Err(Error::new(ErrorCode::InvalidArgument));
    }
    let to_ns = anchor_ns
        .checked_add(1)
        .ok_or_else(|| Error::new(ErrorCode::InvalidArgument))?;
    let mut current =
        TemporalLadder::list_windows(snapshot, TemporalLevel::Week, anchor_ns, to_ns, 1)?
            .into_iter()
            .next();
    let mut windows = Vec::with_capacity(4);
    while let Some(window) = current {
        windows.push(window);
        current = TemporalLadder::open_window(snapshot, window, 1024)?
            .into_iter()
            .find(|child| child.start_ns <= anchor_ns && anchor_ns < child.end_ns);
    }
    let members = windows.last().map_or_else(
        || Ok(Vec::new()),
        |window| TemporalLadder::resolve_members(snapshot, *window, member_limit),
    )?;
    Ok(TimelineRecall { windows, members })
}
