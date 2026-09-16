#![allow(clippy::missing_errors_doc)]

use hm_cortex::citations::{
    CitationClaim, CitationError, FrozenCandidate, FrozenCandidateSet, SourceKind,
};
use hm_schema::events::Authority;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct CitationResult {
    pub attempted: usize,
    pub accepted: usize,
    pub dropped: usize,
    pub escaped_invalid: usize,
}

impl CitationResult {
    #[must_use]
    pub fn drop_rate(self) -> f64 {
        if self.attempted == 0 {
            0.0
        } else {
            self.dropped as f64 / self.attempted as f64
        }
    }
}

pub fn run() -> Result<CitationResult, Box<dyn std::error::Error>> {
    let source = b"Alice wrote \"rotate 5 keys\" after the replica caught up.";
    let frozen = FrozenCandidateSet::new(vec![candidate(7, source)])
        .map_err(|error| format!("invalid citation fixture: {error:?}"))?;
    let valid = CitationClaim {
        lsn: 7,
        byte_start: 0,
        byte_end: u32::try_from(source.len())?,
        quote: b"rotate 5 keys".to_vec(),
    };
    let cases = [
        (
            b"Alice wrote \"rotate 5 keys\" after the replica caught up.".as_slice(),
            vec![valid.clone()],
            true,
        ),
        (
            b"The instruction was invented.".as_slice(),
            vec![CitationClaim {
                quote: b"invented instruction".to_vec(),
                ..valid.clone()
            }],
            false,
        ),
        (
            b"Alice wrote \"rotate 9 keys\" after the replica caught up.".as_slice(),
            vec![valid.clone()],
            false,
        ),
        (
            b"The replica caught up.".as_slice(),
            vec![CitationClaim {
                byte_start: u32::try_from(source.len())?,
                byte_end: u32::try_from(source.len() + 4)?,
                quote: b"caught up".to_vec(),
                ..valid
            }],
            false,
        ),
    ];
    let mut result = CitationResult::default();
    for (derived, claims, expected_valid) in cases {
        result.attempted += 1;
        let outcome = frozen.validate(derived, &claims);
        match (outcome, expected_valid) {
            (Ok(validated), true) => {
                if validated.ranges.is_empty() {
                    return Err("valid citation emitted no ranges".into());
                }
                result.accepted += 1;
            }
            (Err(error), false)
                if matches!(
                    error,
                    CitationError::QuoteOutsideRange(_)
                        | CitationError::UncitedDerivedQuote(_)
                        | CitationError::InvalidRange(_)
                ) =>
            {
                result.dropped += 1;
            }
            (Ok(_), false) => result.escaped_invalid += 1,
            (Err(error), true) => return Err(format!("valid citation dropped: {error:?}").into()),
            (Err(error), false) => {
                return Err(format!("unexpected citation error: {error:?}").into());
            }
        }
    }
    Ok(result)
}

fn candidate(lsn: u64, content: &[u8]) -> FrozenCandidate {
    FrozenCandidate {
        lsn,
        conversation: [1; 16],
        source_root: [2; 32],
        content: content.to_vec(),
        kind: SourceKind::Declarative,
        authority: Authority::UserAsserted,
    }
}
