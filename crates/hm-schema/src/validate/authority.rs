#![allow(clippy::missing_errors_doc)]

use crate::event::{EventHistory, HistorySource};
use crate::events::Authority;
use hm_core::{Error, ErrorCode, LSN};

#[must_use]
pub const fn observed(authority: Authority) -> bool {
    matches!(
        authority,
        Authority::ToolObserved | Authority::ExternalObserved | Authority::RuntimeFact
    )
}

pub fn validate_observed_evidence(lsns: &[u64], history: &impl EventHistory) -> Result<(), Error> {
    if lsns.is_empty() {
        return Err(Error::new(ErrorCode::CitationInvalid));
    }
    for raw_lsn in lsns {
        let lsn = LSN::new(*raw_lsn);
        if *raw_lsn == 0
            || history.source_at(lsn) != HistorySource::LedgerEvent
            || !history.authority_at(lsn).is_some_and(observed)
        {
            return Err(Error::new(ErrorCode::CitationInvalid).at_lsn(lsn));
        }
    }
    Ok(())
}

pub fn validate_optional_observed_evidence(
    lsns: Option<&[u64]>,
    history: &impl EventHistory,
    legacy_evidence_allowed: bool,
) -> Result<(), Error> {
    match lsns {
        Some(lsns) => validate_observed_evidence(lsns, history),
        None if legacy_evidence_allowed => Ok(()),
        None => Err(Error::new(ErrorCode::CitationInvalid)),
    }
}
