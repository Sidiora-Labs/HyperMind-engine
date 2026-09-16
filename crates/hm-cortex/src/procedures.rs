#![allow(clippy::missing_errors_doc)]

use hm_core::{Error, ErrorCode};
use hm_schema::events::{Authority, ProcedureMined, ProcedureSupport};
use std::collections::BTreeSet;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Episode {
    pub tool_call_lsn: u64,
    pub tool_call_id: Vec<u8>,
    pub tool_result_lsn: u64,
    pub tool_result_call_id: Vec<u8>,
    pub effect_lsn: u64,
    pub effect_tool_call_lsn: u64,
    pub effect_id: Vec<u8>,
    pub outcome_lsn: u64,
    pub outcome_effect_id: Vec<u8>,
    pub loop_closed_lsn: u64,
    pub loop_evidence_lsns: Vec<u64>,
    pub source_root: Vec<u8>,
    pub conversation: Vec<u8>,
    pub successful: bool,
}

pub fn mine(
    procedure_id: &[u8],
    strategy: &str,
    expected_outcomes: Vec<String>,
    preconditions: Vec<String>,
    episodes: &[Episode],
) -> Result<ProcedureMined, Error> {
    if procedure_id.is_empty() || strategy.is_empty() || episodes.is_empty() {
        return Err(Error::new(ErrorCode::InvalidArgument));
    }
    let mut supports = Vec::new();
    let mut failures = Vec::new();
    for episode in episodes {
        if !(episode.tool_call_lsn < episode.tool_result_lsn
            && episode.tool_result_lsn < episode.effect_lsn
            && episode.effect_lsn < episode.outcome_lsn
            && episode.outcome_lsn < episode.loop_closed_lsn)
            || episode.tool_call_id != episode.tool_result_call_id
            || episode.effect_tool_call_lsn != episode.tool_call_lsn
            || episode.effect_id != episode.outcome_effect_id
            || !episode.loop_evidence_lsns.contains(&episode.outcome_lsn)
            || episode.source_root.is_empty()
            || episode.conversation.len() != 16
        {
            return Err(Error::new(ErrorCode::OrderingViolation));
        }
        if episode.successful {
            supports.push(ProcedureSupport {
                source_root: episode.source_root.clone(),
                conversation: episode.conversation.clone(),
                episode_lsn: episode.loop_closed_lsn,
            });
        } else {
            failures.push(episode.outcome_lsn);
        }
    }
    Ok(ProcedureMined {
        procedure_id: procedure_id.to_vec(),
        strategy: strategy.to_owned(),
        expected_outcomes,
        preconditions,
        supports,
        failures: (!failures.is_empty()).then_some(failures.clone()),
        counterexamples: (!failures.is_empty()).then_some(failures),
    })
}

#[must_use]
pub fn supported(procedure: &ProcedureMined) -> bool {
    procedure
        .supports
        .iter()
        .map(|support| support.source_root.as_slice())
        .collect::<BTreeSet<_>>()
        .len()
        >= 3
        && procedure
            .supports
            .iter()
            .map(|support| support.conversation.as_slice())
            .collect::<BTreeSet<_>>()
            .len()
            >= 2
}

#[must_use]
pub fn can_adopt(procedure: &ProcedureMined, authority: Authority) -> bool {
    supported(procedure) && authority == Authority::UserAsserted
}
