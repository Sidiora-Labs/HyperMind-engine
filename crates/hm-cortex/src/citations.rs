#![allow(clippy::missing_errors_doc)]

use crate::authority::{CitedSource, derive_authority};
use hm_schema::events::{Authority, ProvenanceRange};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SourceKind {
    Declarative,
    Reflective,
    Speculation,
    Interrogative,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FrozenCandidate {
    pub lsn: u64,
    pub conversation: [u8; 16],
    pub source_root: [u8; 32],
    pub content: Vec<u8>,
    pub kind: SourceKind,
    pub authority: Authority,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CitationClaim {
    pub lsn: u64,
    pub byte_start: u32,
    pub byte_end: u32,
    pub quote: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidatedCitations {
    pub ranges: Vec<ProvenanceRange>,
    pub authority: Authority,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CitationError {
    EmptyCandidates,
    DuplicateLsn(u64),
    MissingCitation,
    UnknownLsn(u64),
    InvalidRange(u64),
    QuoteOutsideRange(u64),
    UncitedDerivedQuote(Vec<u8>),
}

#[derive(Clone, Debug)]
pub struct FrozenCandidateSet {
    candidates: BTreeMap<u64, FrozenCandidate>,
}

impl FrozenCandidateSet {
    pub fn new(candidates: Vec<FrozenCandidate>) -> Result<Self, CitationError> {
        if candidates.is_empty() {
            return Err(CitationError::EmptyCandidates);
        }
        let mut indexed = BTreeMap::new();
        for candidate in candidates {
            let lsn = candidate.lsn;
            if lsn == 0 || indexed.insert(lsn, candidate).is_some() {
                return Err(CitationError::DuplicateLsn(lsn));
            }
        }
        Ok(Self {
            candidates: indexed,
        })
    }

    pub fn candidates(&self) -> impl Iterator<Item = &FrozenCandidate> {
        self.candidates.values()
    }

    #[must_use]
    pub fn get(&self, lsn: u64) -> Option<&FrozenCandidate> {
        self.candidates.get(&lsn)
    }

    pub fn validate(
        &self,
        derived: &[u8],
        claims: &[CitationClaim],
    ) -> Result<ValidatedCitations, CitationError> {
        if claims.is_empty() {
            return Err(CitationError::MissingCitation);
        }
        let mut ranges = Vec::with_capacity(claims.len());
        let mut validated_quotes = Vec::with_capacity(claims.len());
        for claim in claims {
            let source = self
                .get(claim.lsn)
                .ok_or(CitationError::UnknownLsn(claim.lsn))?;
            let start = usize::try_from(claim.byte_start)
                .map_err(|_| CitationError::InvalidRange(claim.lsn))?;
            let end = usize::try_from(claim.byte_end)
                .map_err(|_| CitationError::InvalidRange(claim.lsn))?;
            let cited = source
                .content
                .get(start..end)
                .filter(|range| !range.is_empty())
                .ok_or(CitationError::InvalidRange(claim.lsn))?;
            if claim.quote.is_empty()
                || !cited
                    .windows(claim.quote.len())
                    .any(|window| window == claim.quote)
            {
                return Err(CitationError::QuoteOutsideRange(claim.lsn));
            }
            validated_quotes.push(claim.quote.clone());
            ranges.push(ProvenanceRange {
                first_lsn: claim.lsn,
                last_lsn: claim.lsn,
                byte_start: claim.byte_start,
                byte_end: claim.byte_end,
            });
        }
        for quoted in quoted_spans(derived) {
            if !validated_quotes.iter().any(|value| value == &quoted) {
                return Err(CitationError::UncitedDerivedQuote(quoted));
            }
        }
        let authority = verbatim_authority(self, derived, claims);
        Ok(ValidatedCitations { ranges, authority })
    }
}

fn verbatim_authority(
    candidates: &FrozenCandidateSet,
    derived: &[u8],
    claims: &[CitationClaim],
) -> Authority {
    for claim in claims {
        let Some(source) = candidates.get(claim.lsn) else {
            continue;
        };
        let Ok(start) = usize::try_from(claim.byte_start) else {
            continue;
        };
        let Ok(end) = usize::try_from(claim.byte_end) else {
            continue;
        };
        let cited = CitedSource {
            authority: source.authority,
            bytes: &source.content,
            cited_range: start..end,
        };
        let authority = derive_authority(&cited, derived);
        if authority != Authority::DerivedInference {
            return authority;
        }
    }
    Authority::DerivedInference
}

fn quoted_spans(bytes: &[u8]) -> Vec<Vec<u8>> {
    let mut output = Vec::new();
    let mut index = 0;
    while index < bytes.len() {
        let (open, close) = if bytes[index] == b'"' {
            (&bytes[index..=index], &b"\""[..])
        } else if bytes[index..].starts_with("“".as_bytes()) {
            ("“".as_bytes(), "”".as_bytes())
        } else {
            index += 1;
            continue;
        };
        let start = index + open.len();
        let Some(relative_end) = bytes[start..]
            .windows(close.len())
            .position(|window| window == close)
        else {
            break;
        };
        let end = start + relative_end;
        if end > start {
            output.push(bytes[start..end].to_vec());
        }
        index = end + close.len();
    }
    output
}
